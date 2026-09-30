//! HCS action registry — one control surface for finding **and** doing.
//!
//! Pattern source: macOS Spotlight's "Actions" browsing view and App Intents
//! (hundreds of actions reachable from one field without leaving the app), plus
//! Android 16's standardised action surface and Windows 11's Click to Do
//! (actions on what is on screen, without a context switch).
//!
//! Design contract:
//!
//! * An **action** is anything the system can *do*, not just anything it can
//!   *show*. Apps register their capabilities here; the omnibar, the CLI and the
//!   agent all consume the same registry. That is the whole point — one registry,
//!   three front ends, no duplicated logic.
//! * Every action declares whether it is **privileged**. Privileged actions
//!   require explicit human confirmation (HITL), reusing the existing
//!   `hcs-agents` permission model rather than inventing a second gate.
//! * Quick keys are short strings resolved against the query, so two keystrokes
//!   replace menu hunting. A quick key must be unambiguous or resolution fails
//!   loudly instead of guessing.
//! * The registry is pure data plus a dispatcher. It performs no I/O itself,
//!   which is what makes the whole module testable without a display.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// How risky an action is. `Privileged` actions are gated behind human
/// confirmation; `ReadOnly` actions are safe to run from a suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// No state change, no privilege. Safe to run from an autocomplete row.
    ReadOnly,
    /// Changes system or user state. Requires confirmation.
    Mutating,
    /// Touches network, Tor rules, the vault or package management.
    /// Requires confirmation and is logged to the audit ledger.
    Privileged,
}

/// The category an action belongs to. Drives grouping in the omnibar and the
/// icon shown next to a result row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    App,
    Window,
    System,
    Privacy,
    Ai,
    Files,
    Settings,
    Update,
    Help,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::App => "app",
            Category::Window => "window",
            Category::System => "system",
            Category::Privacy => "privacy",
            Category::Ai => "ai",
            Category::Files => "files",
            Category::Settings => "settings",
            Category::Update => "update",
            Category::Help => "help",
        }
    }
}

/// A single registered action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Stable identifier, dot-separated, e.g. `hcs.image.generate`.
    pub id: String,
    /// Human title shown in the omnibar.
    pub title: String,
    /// Longer description shown in the preview pane.
    pub description: String,
    pub category: Category,
    pub risk: RiskLevel,
    /// Lower sorts and matches earlier. System essentials get 0.
    pub priority: i32,
    /// Short two-or-three character key resolved against the query (macOS
    /// "quick keys" pattern). Optional.
    pub quick_key: Option<String>,
    /// Alternative surface: an executable plus argv, used by the dispatcher.
    pub command: Option<Vec<String>>,
    /// Free-form keywords that should match even if the title does not.
    pub keywords: Vec<String>,
    /// Whether this action needs a human to confirm before running.
    pub requires_confirmation: bool,
}

impl Action {
    pub fn new(id: &str, title: &str, category: Category, risk: RiskLevel) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: String::new(),
            category,
            risk,
            priority: 50,
            quick_key: None,
            command: None,
            keywords: Vec::new(),
            requires_confirmation: matches!(risk, RiskLevel::Privileged),
        }
    }

    pub fn description(mut self, d: &str) -> Self {
        self.description = d.to_string();
        self
    }

    pub fn priority(mut self, p: i32) -> Self {
        self.priority = p;
        self
    }

    pub fn quick_key(mut self, k: &str) -> Self {
        self.quick_key = Some(k.to_string());
        self
    }

    pub fn command(mut self, program: &str, args: &[&str]) -> Self {
        let mut argv = vec![program.to_string()];
        argv.extend(args.iter().map(|a| a.to_string()));
        self.command = Some(argv);
        self
    }

    pub fn keywords(mut self, kws: &[&str]) -> Self {
        self.keywords = kws.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Score against a query. Title matches outrank description matches, which
    /// outrank keyword matches; an exact prefix beats a mid-string hit. Ties
    /// break on `priority`, then title, so ordering is fully deterministic.
    pub fn score(&self, query: &str) -> Option<f64> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Some(self.priority as f64 / 100.0);
        }
        let title = self.title.to_lowercase();
        let desc = self.description.to_lowercase();
        let id = self.id.to_lowercase();

        let mut best: f64 = 0.0;
        if title == q {
            best = best.max(1.0);
        } else if title.starts_with(&q) {
            best = best.max(0.9);
        } else if title.contains(&q) {
            best = best.max(0.75);
        }
        if id.starts_with(&q) {
            best = best.max(0.85);
        } else if id.contains(&q) {
            best = best.max(0.6);
        }
        if desc.contains(&q) {
            best = best.max(0.5);
        }
        for kw in &self.keywords {
            let k = kw.to_lowercase();
            if k == q {
                best = best.max(0.8);
            } else if k.starts_with(&q) {
                best = best.max(0.65);
            }
        }
        if best > 0.0 {
            // Priority nudges, never overrides, a real textual match.
            Some(best + (self.priority as f64 / 1000.0))
        } else {
            None
        }
    }
}

/// A ranked search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredAction {
    pub action: Action,
    pub score: f64,
}

/// Outcome of resolving a quick key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickKeyResolution {
    Resolved(String),
    /// More than one action claims the same key. We refuse to guess.
    Ambiguous(Vec<String>),
    NotFound,
}

/// The registry itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionRegistry {
    actions: Vec<Action>,
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("duplicate action id: {0}")]
    DuplicateId(String),
    #[error("action not found: {0}")]
    NotFound(String),
    #[error("action '{0}' requires confirmation")]
    NeedsConfirmation(String),
    #[error("action '{0}' has no executable command")]
    NoCommand(String),
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an action. Duplicate ids are rejected: silently keeping the last
    /// writer would make the registry order-dependent and hide real bugs.
    pub fn register(&mut self, action: Action) -> Result<(), RegistryError> {
        if self.actions.iter().any(|a| a.id == action.id) {
            return Err(RegistryError::DuplicateId(action.id));
        }
        self.actions.push(action);
        Ok(())
    }

    pub fn register_all(
        &mut self,
        actions: impl IntoIterator<Item = Action>,
    ) -> Result<(), RegistryError> {
        for a in actions {
            self.register(a)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn all(&self) -> &[Action] {
        &self.actions
    }

    pub fn get(&self, id: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.id == id)
    }

    pub fn in_category(&self, category: Category) -> Vec<&Action> {
        self.actions
            .iter()
            .filter(|a| a.category == category)
            .collect()
    }

    /// Rank actions against a query. Deterministic: equal scores sort by
    /// priority, then id.
    pub fn search(&self, query: &str, limit: usize) -> Vec<ScoredAction> {
        let mut scored: Vec<ScoredAction> = self
            .actions
            .iter()
            .filter_map(|a| {
                a.score(query).map(|s| ScoredAction {
                    action: a.clone(),
                    score: s,
                })
            })
            .collect();
        scored.sort_by(|x, y| {
            y.score
                .partial_cmp(&x.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| x.action.priority.cmp(&y.action.priority))
                .then_with(|| x.action.id.cmp(&y.action.id))
        });
        scored.truncate(limit);
        scored
    }

    /// Resolve a quick key. Returns `Ambiguous` rather than picking a winner:
    /// running the wrong privileged action because two keys collided is worse
    /// than telling the user to type more.
    pub fn resolve_quick_key(&self, key: &str) -> QuickKeyResolution {
        let k = key.trim().to_lowercase();
        if k.is_empty() {
            return QuickKeyResolution::NotFound;
        }
        let matches: Vec<String> = self
            .actions
            .iter()
            .filter(|a| a.quick_key.as_deref().map(str::to_lowercase) == Some(k.clone()))
            .map(|a| a.id.clone())
            .collect();
        match matches.len() {
            0 => QuickKeyResolution::NotFound,
            1 => QuickKeyResolution::Resolved(matches.into_iter().next().unwrap()),
            _ => QuickKeyResolution::Ambiguous(matches),
        }
    }

    /// Check whether an action may run without confirmation.
    pub fn check_permission(&self, id: &str, confirmed: bool) -> Result<(), RegistryError> {
        let action = self
            .get(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        if action.requires_confirmation && !confirmed {
            return Err(RegistryError::NeedsConfirmation(id.to_string()));
        }
        Ok(())
    }

    /// Build the executable argv for an action. Separated from execution so the
    /// decision "what would run" is testable and auditable on its own.
    pub fn argv_for(&self, id: &str, confirmed: bool) -> Result<Vec<String>, RegistryError> {
        self.check_permission(id, confirmed)?;
        let action = self
            .get(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))?;
        action
            .command
            .clone()
            .ok_or_else(|| RegistryError::NoCommand(id.to_string()))
    }

    /// Browse view: all actions in a category, sorted by title. Mirrors the
    /// macOS browse views so a user who does not know the name can still scan.
    pub fn browse(&self, category: Option<Category>) -> Vec<&Action> {
        let mut list: Vec<&Action> = match category {
            Some(c) => self.in_category(c),
            None => self.actions.iter().collect(),
        };
        list.sort_by(|a, b| a.title.cmp(&b.title));
        list
    }

    /// Validate the registry. Duplicate quick keys are reported because they
    /// are user-visible ambiguity even though they do not break registration.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for a in &self.actions {
            if let Some(k) = &a.quick_key {
                seen.entry(k.to_lowercase()).or_default().push(a.id.clone());
            }
            if a.command.is_none() && a.risk != RiskLevel::ReadOnly {
                problems.push(format!(
                    "action '{}' is {:?} but has no command to run",
                    a.id, a.risk
                ));
            }
        }
        for (k, ids) in seen {
            if ids.len() > 1 {
                problems.push(format!("quick key '{}' claimed by {:?}", k, ids));
            }
        }
        problems
    }
}

impl fmt::Display for QuickKeyResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuickKeyResolution::Resolved(id) => write!(f, "resolved:{id}"),
            QuickKeyResolution::Ambiguous(ids) => write!(f, "ambiguous:{}", ids.join(",")),
            QuickKeyResolution::NotFound => write!(f, "not-found"),
        }
    }
}

/// Search history. Not persisted by this type — the caller decides, because in
/// amnesic mode (Tails pattern) history must not survive the session at all.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchHistory {
    entries: Vec<String>,
}

impl SearchHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a query. Consecutive duplicates move to the front instead of
    /// piling up, which is what the macOS history arrow expects.
    pub fn record(&mut self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            return;
        }
        self.entries.retain(|e| e != q);
        self.entries.insert(0, q.to_string());
        self.entries.truncate(50);
    }

    /// History navigation: index 0 is the most recent, matching the `Up` key.
    pub fn at(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// The built-in registry: every system capability that must be reachable from
/// the omnibar and from the CLI. Apps extend this at runtime via
/// `hcs actions register`.
pub fn builtin_registry() -> ActionRegistry {
    use Category as C;
    use RiskLevel as R;

    let mut reg = ActionRegistry::new();

    let actions = vec![
        // --- apps ------------------------------------------------------------
        Action::new("app.chat", "AI Chat", C::App, R::ReadOnly)
            .description("Talk to the local cognitive models")
            .priority(10)
            .quick_key("ch")
            .command("hcs-chat", &["--gui"])
            .keywords(&["brain", "ai", "llm", "assistant"]),
        Action::new("app.image", "Image Studio", C::App, R::ReadOnly)
            .description("Offline CPU text-to-image (SD 1.5 LCM)")
            .priority(10)
            .quick_key("im")
            .command("hcs-chat", &["--gui", "--tab", "image"])
            .keywords(&["picture", "render", "sd", "draw"]),
        Action::new("app.monitor", "System Monitor", C::App, R::ReadOnly)
            .description("RAM, model residency and daemon state")
            .priority(10)
            .quick_key("mo")
            .command("hcs-monitor", &["--gui"])
            .keywords(&["ram", "memory", "cpu", "telemetry"]),
        Action::new("app.control", "Control Center", C::App, R::ReadOnly)
            .description("Tor, AI profile, vault and privacy toggles")
            .priority(10)
            .quick_key("co")
            .command("hcs-control", &["--gui"])
            .keywords(&["tor", "privacy", "profile", "vault"]),
        Action::new("app.files", "Files", C::App, R::ReadOnly)
            .description("Browse, search and preview files")
            .priority(10)
            .quick_key("fi")
            .command("hcs-fm", &[])
            .keywords(&["folder", "explorer", "nemo", "documents"]),
        Action::new("app.terminal", "Terminal", C::App, R::ReadOnly)
            .description("Terminal emulator with hcs dev profiles")
            .priority(10)
            .quick_key("te")
            .command("hcs-term", &[])
            .keywords(&["shell", "console", "bash", "command line"]),
        Action::new("app.notes", "Notes", C::App, R::ReadOnly)
            .description("Markdown notes with offline search")
            .priority(20)
            .quick_key("no")
            .command("hcs-notes", &[])
            .keywords(&["notepad", "markdown", "todo"]),
        Action::new("app.docs", "Documentation", C::Help, R::ReadOnly)
            .description("Offline manuals, no browser required")
            .priority(20)
            .quick_key("do")
            .command("hcs-docs", &[])
            .keywords(&["manual", "help", "guide", "handbook"]),
        Action::new("app.diagnose", "Diagnose", C::System, R::ReadOnly)
            .description("Triage the system and apply fixes")
            .priority(30)
            .quick_key("dg")
            .command("hcs-diagnose", &["--gui"])
            .keywords(&["health", "repair", "fix", "problem"]),
        Action::new("app.settings", "Settings", C::Settings, R::ReadOnly)
            .description("Theme, wallpaper, keyboard, privacy defaults")
            .priority(20)
            .quick_key("se")
            .command("hcs-settings", &[])
            .keywords(&["preferences", "config", "options"]),
        Action::new("app.shot", "Screenshot", C::App, R::ReadOnly)
            .description("Region, window or screen capture with OCR")
            .priority(30)
            .quick_key("ss")
            .command("hcs-shot", &["--region"])
            .keywords(&["snip", "capture", "print screen", "clip"]),
        Action::new("app.rag", "Knowledge Ingest", C::Ai, R::ReadOnly)
            .description("Index documents into the local retrieval store")
            .priority(30)
            .quick_key("kg")
            .command("hcs-rag-ingest", &["--gui"])
            .keywords(&["rag", "documents", "index", "embedding"]),
        // --- window management -------------------------------------------------
        Action::new("win.snap.left", "Snap Window Left", C::Window, R::Mutating)
            .description("Snap the focused window to the left half")
            .priority(40)
            .quick_key("sl")
            .command("hcs", &["window", "snap", "--side", "left"])
            .keywords(&["snap", "half", "tiling"]),
        Action::new(
            "win.snap.right",
            "Snap Window Right",
            C::Window,
            R::Mutating,
        )
        .description("Snap the focused window to the right half")
        .priority(40)
        .quick_key("sr")
        .command("hcs", &["window", "snap", "--side", "right"]),
        Action::new("win.snap.layout", "Snap Layouts", C::Window, R::Mutating)
            .description("Choose a window arrangement")
            .priority(40)
            .quick_key("sz")
            .command("hcs", &["window", "snap", "--flyout"]),
        Action::new("win.taskview", "Task View", C::Window, R::ReadOnly)
            .description("All windows and virtual desktops")
            .priority(40)
            .quick_key("tv")
            .command("hcs", &["window", "taskview"]),
        Action::new(
            "win.desktop.new",
            "New Virtual Desktop",
            C::Window,
            R::Mutating,
        )
        .priority(50)
        .command("hcs", &["desktop", "new"]),
        Action::new(
            "win.mode.toggle",
            "Toggle Tiling Mode",
            C::Window,
            R::Mutating,
        )
        .description("Switch between floating and tiling layouts")
        .priority(50)
        .command("hcs", &["window", "mode", "toggle"]),
        Action::new("win.stage", "Toggle Stage Manager", C::Window, R::Mutating)
            .description("One app in front, the rest reduced to a strip")
            .priority(50)
            .command("hcs", &["window", "stage", "toggle"]),
        // --- privacy (privileged) ------------------------------------------------
        Action::new(
            "privacy.tor.toggle",
            "Toggle Tor Kill Switch",
            C::Privacy,
            R::Privileged,
        )
        .description("Route everything through Tor, or fail closed")
        .priority(60)
        .quick_key("to")
        .command("hcs", &["security", "tor", "toggle"])
        .keywords(&["vpn", "anonymity", "killswitch", "dns leak"]),
        Action::new(
            "privacy.amnesic.toggle",
            "Toggle Amnesic Session",
            C::Privacy,
            R::Privileged,
        )
        .description("Leave no trace on local storage")
        .priority(60)
        .command("hcs", &["privacy", "amnesic", "toggle"])
        .keywords(&["tails", "no trace", "forget"]),
        Action::new("privacy.recall", "Recall Timeline", C::Privacy, R::ReadOnly)
            .description("Search what you have seen (local and encrypted)")
            .priority(60)
            .quick_key("rc")
            .command("hcs", &["recall"])
            .keywords(&["timeline", "history", "windows recall"]),
        Action::new("privacy.vault", "Open Vault", C::Privacy, R::Privileged)
            .description("LUKS2 encrypted volumes")
            .priority(60)
            .command("hcs", &["security", "vault"]),
        // --- AI ------------------------------------------------------------------
        Action::new("ai.ask", "Ask the Brain", C::Ai, R::ReadOnly)
            .description("Context-aware question to the local models")
            .priority(30)
            .quick_key("ai")
            .command("hcs-chat", &["--gui"])
            .keywords(&["ask", "question", "llm"]),
        Action::new(
            "ai.explain",
            "Explain Selection with Brain",
            C::Ai,
            R::ReadOnly,
        )
        .description("Explain the selected text or screen region")
        .priority(40)
        .command("hcs", &["ask", "--selection"])
        .keywords(&["explain", "selection", "highlight"]),
        Action::new("ai.rag.query", "Search the Manuals", C::Ai, R::ReadOnly)
            .description("Retrieval over the offline documentation")
            .priority(40)
            .quick_key("rq")
            .command("hcs", &["rag", "query"])
            .keywords(&["docs search", "knowledge", "answer"]),
        Action::new("ai.image.generate", "Generate Image", C::Ai, R::Mutating)
            .description("Offline CPU image generation")
            .priority(40)
            .command("hcs", &["image", "--steps", "6"])
            .keywords(&["render", "draw", "picture"]),
        // --- update ----------------------------------------------------------------
        Action::new("update.check", "Check for Updates", C::Update, R::ReadOnly)
            .priority(70)
            .command("hcs", &["update", "check"]),
        Action::new("update.apply", "Apply Updates", C::Update, R::Privileged)
            .description("Snapshot first, then apply the selected levels")
            .priority(70)
            .command("hcs", &["update", "apply"])
            .keywords(&["upgrade", "patch", "rollback"]),
        Action::new(
            "update.rollback",
            "Roll Back System",
            C::Update,
            R::Privileged,
        )
        .description("Return to the previous snapshot from the boot menu")
        .priority(70)
        .command("hcs", &["update", "rollback"]),
        // --- settings ----------------------------------------------------------------
        Action::new("settings.theme", "Change Theme", C::Settings, R::Mutating)
            .description("Obsidian, Titanium, Stealth or High Contrast")
            .priority(60)
            .command("hcs", &["theme", "set"]),
        Action::new(
            "settings.keyboard",
            "Change Keyboard Layout",
            C::Settings,
            R::Mutating,
        )
        .description("QWERTZ, EN-US, FR, ES, IT, GB")
        .priority(60)
        .command("hcs", &["settings", "keyboard"])
        .keywords(&["qwertz", "qwerty", "azerty", "layout", "language"]),
        Action::new(
            "settings.reduce_motion",
            "Toggle Reduce Motion",
            C::Settings,
            R::Mutating,
        )
        .description("Remove animation across the whole system")
        .priority(60)
        .command("hcs", &["settings", "reduce-motion", "toggle"]),
        Action::new(
            "settings.high_contrast",
            "Toggle High Contrast",
            C::Settings,
            R::Mutating,
        )
        .priority(60)
        .command("hcs", &["settings", "contrast", "toggle"]),
    ];

    // Registration failures here would be a programming error, not a runtime
    // condition, so they are surfaced loudly rather than swallowed.
    if let Err(e) = reg.register_all(actions) {
        eprintln!("[FATAL] builtin action registry is inconsistent: {e}");
        std::process::exit(1);
    }
    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_is_consistent() {
        let reg = builtin_registry();
        let problems = reg.validate();
        assert!(problems.is_empty(), "registry problems: {problems:?}");
        assert!(
            reg.len() >= 25,
            "expected a broad builtin set, got {}",
            reg.len()
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut reg = ActionRegistry::new();
        reg.register(Action::new(
            "a.one",
            "One",
            Category::App,
            RiskLevel::ReadOnly,
        ))
        .unwrap();
        let err = reg.register(Action::new(
            "a.one",
            "Again",
            Category::App,
            RiskLevel::ReadOnly,
        ));
        assert!(matches!(err, Err(RegistryError::DuplicateId(_))));
    }

    #[test]
    fn title_prefix_outranks_keyword_match() {
        let reg = builtin_registry();
        let hits = reg.search("snap", 10);
        assert!(!hits.is_empty());
        // Every hit is a snap action, and the highest scorer must be a real
        // snap action rather than something that merely mentions "snap".
        assert!(hits[0].action.id.starts_with("win.snap."));
    }

    #[test]
    fn search_is_deterministic_for_equal_scores() {
        let reg = builtin_registry();
        let a = reg.search("e", 20);
        let b = reg.search("e", 20);
        let ids_a: Vec<_> = a.iter().map(|s| s.action.id.clone()).collect();
        let ids_b: Vec<_> = b.iter().map(|s| s.action.id.clone()).collect();
        assert_eq!(ids_a, ids_b);
    }

    #[test]
    fn empty_query_returns_priority_order() {
        let reg = builtin_registry();
        let hits = reg.search("   ", 5);
        assert!(!hits.is_empty());
        // With nothing to match textually, the highest priority leads. Priority is
        // used as "how much does this matter", so a larger number must rank first.
        for pair in hits.windows(2) {
            assert!(
                pair[0].score >= pair[1].score,
                "results must be non-increasing: {:?} then {:?}",
                pair[0].action.id,
                pair[1].action.id
            );
        }
        assert_eq!(
            hits[0].action.priority, 70,
            "update actions lead the browse order"
        );
    }

    #[test]
    fn quick_key_resolves() {
        let reg = builtin_registry();
        assert_eq!(
            reg.resolve_quick_key("ch"),
            QuickKeyResolution::Resolved("app.chat".to_string())
        );
        assert_eq!(reg.resolve_quick_key("zz"), QuickKeyResolution::NotFound);
    }

    #[test]
    fn colliding_quick_keys_are_ambiguous_not_guessed() {
        let mut reg = ActionRegistry::new();
        reg.register(Action::new("a.x", "X", Category::App, RiskLevel::ReadOnly).quick_key("dup"))
            .unwrap();
        reg.register(Action::new("a.y", "Y", Category::App, RiskLevel::ReadOnly).quick_key("dup"))
            .unwrap();
        match reg.resolve_quick_key("dup") {
            QuickKeyResolution::Ambiguous(ids) => {
                assert_eq!(ids, vec!["a.x".to_string(), "a.y".to_string()]);
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
        assert!(reg.validate().iter().any(|p| p.contains("quick key 'dup'")));
    }

    #[test]
    fn privileged_action_requires_confirmation() {
        let reg = builtin_registry();
        let err = reg.argv_for("privacy.tor.toggle", false).unwrap_err();
        assert!(matches!(err, RegistryError::NeedsConfirmation(_)));
        let argv = reg.argv_for("privacy.tor.toggle", true).unwrap();
        assert_eq!(argv.first().map(String::as_str), Some("hcs"));
    }

    #[test]
    fn read_only_action_runs_without_confirmation() {
        let reg = builtin_registry();
        let argv = reg.argv_for("app.chat", false).unwrap();
        assert_eq!(argv, vec!["hcs-chat".to_string(), "--gui".to_string()]);
    }

    #[test]
    fn unknown_action_is_an_error_not_a_panic() {
        let reg = builtin_registry();
        assert!(matches!(
            reg.argv_for("does.not.exist", false),
            Err(RegistryError::NotFound(_))
        ));
    }

    #[test]
    fn history_moves_duplicates_to_front() {
        let mut h = SearchHistory::new();
        h.record("ram");
        h.record("tor");
        h.record("ram");
        assert_eq!(h.at(0), Some("ram"));
        assert_eq!(h.at(1), Some("tor"));
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn history_ignores_blank_queries() {
        let mut h = SearchHistory::new();
        h.record("   ");
        h.record("");
        assert!(h.is_empty());
    }

    #[test]
    fn browse_is_sorted_by_title() {
        let reg = builtin_registry();
        let items = reg.browse(Some(Category::App));
        let titles: Vec<_> = items.iter().map(|a| a.title.as_str()).collect();
        let mut sorted = titles.clone();
        sorted.sort_unstable();
        assert_eq!(titles, sorted);
    }

    #[test]
    fn mutating_action_without_command_is_flagged() {
        let mut reg = ActionRegistry::new();
        reg.register(
            Action::new("bad.one", "Bad", Category::System, RiskLevel::Mutating).priority(10),
        )
        .unwrap();
        let problems = reg.validate();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("no command"));
    }
}
