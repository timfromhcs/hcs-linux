//! HCS Linux â€” Neural Glass GUI foundation (plan: docs/GUI_BUILD_PLAN.md Â§2 P0).
//!
//! Slint is used under the **Slint Royalty-free License 2.0**
//! (`LicenseRef-Slint-Royalty-free-2.0`), which is the copyleft-free option of
//! Slint's triple license and therefore keeps HCS Linux under Apache-2.0.
//! The Royalty-free license requires disclosing the use of Slint; every app
//! built on this crate renders `disclosure()` in its About dialog and the
//! system Settings app lists it in the third-party section.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::rc::Rc;

slint::include_modules!();

pub mod harness;
pub use harness::{render_to_png, HeadlessPlatform, RenderError};

// `include_modules!()` above brings the generated types (`AppFrame`, `Palette`,
// `Presets`, `WidgetGallery`, ...) into the crate root, so app crates and the
// render harness can simply `use hcs_ui::AppFrame;`.

/// Budget ceilings from the master plan RAM matrix (docs/V1_STABLE_RELEASE_MASTER_PLAN.md Â§5).
pub const IDLE_BUDGET_MB: f32 = 6144.0;
pub const PEAK_BUDGET_MB: f32 = 8192.0;
/// Per-application ceiling for GUI apps.
pub const APP_RSS_BUDGET_MB: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreset {
    /// Obsidian Neural Gradient â€” default dark theme.
    Obsidian,
    /// Frosted Titanium â€” light architectural theme.
    Titanium,
    /// Cybernetic Stealth â€” Security Lab theme.
    Stealth,
}

impl ThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemePreset::Obsidian => "obsidian",
            ThemePreset::Titanium => "titanium",
            ThemePreset::Stealth => "stealth",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "obsidian" => Some(ThemePreset::Obsidian),
            "titanium" => Some(ThemePreset::Titanium),
            "stealth" => Some(ThemePreset::Stealth),
            _ => None,
        }
    }
}

/// Mandatory Slint attribution text (Royalty-free License 2.0 disclosure).
pub fn disclosure() -> &'static str {
    "This application is built with Slint (https://slint.dev), \
     used under the Slint Royalty-free License 2.0."
}

/// Persist the active theme so the shell (QML) and the Slint apps agree.
pub fn theme_config_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(dir).join("hcs/theme.json")
    } else {
        std::path::PathBuf::from("/etc/hcs/theme.json")
    }
}

/// Booleans that select the active Neural Glass preset. The palette itself is
/// resolved inside `AppFrame` (`theme-titanium` / `theme-stealth`), so plain
/// `bool` bindings are all that is needed — those are always code-generated,
/// which keeps the theme system testable from `cargo test`.
pub fn preset_flags(preset: ThemePreset) -> (bool, bool) {
    match preset {
        ThemePreset::Obsidian => (false, false),
        ThemePreset::Titanium => (true, false),
        ThemePreset::Stealth => (false, true),
    }
}

/// Implemented for every generated window type so the theme can be applied
/// without knowing the concrete type.
pub trait ThemeTarget {
    fn apply_theme_flags(&self, titanium: bool, stealth: bool);
}

/// Implement [`ThemeTarget`] for a generated Slint window type.
/// Usage: `hcs_ui::theme_target!(hcs_ui::WidgetGallery);`
#[macro_export]
macro_rules! theme_target {
    ($t:ty) => {
        impl $crate::ThemeTarget for $t {
            fn apply_theme_flags(&self, titanium: bool, stealth: bool) {
                self.set_theme_titanium(titanium);
                self.set_theme_stealth(stealth);
            }
        }
    };
}

theme_target!(WidgetGallery);

/// Read the persisted theme preset (defaults to Obsidian).
pub fn load_theme() -> ThemePreset {
    let p = theme_config_path();
    if let Ok(txt) = std::fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            if let Some(name) = v.get("theme").and_then(|x| x.as_str()) {
                if let Some(preset) = ThemePreset::from_str_opt(name) {
                    return preset;
                }
            }
        }
    }
    ThemePreset::Obsidian
}
pub fn save_theme(preset: ThemePreset) -> std::io::Result<()> {
    let p = theme_config_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({ "theme": preset.as_str() });
    std::fs::write(
        &p,
        serde_json::to_string_pretty(&payload).unwrap_or_default(),
    )
}

/// RAM budget helper shared by monitor + image studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamBudget {
    pub used_mb: u64,
    pub idle_budget_mb: u64,
    pub peak_budget_mb: u64,
}

impl RamBudget {
    pub fn new(used_mb: u64) -> Self {
        Self {
            used_mb,
            idle_budget_mb: IDLE_BUDGET_MB as u64,
            peak_budget_mb: PEAK_BUDGET_MB as u64,
        }
    }

    pub fn within_idle(&self) -> bool {
        self.used_mb <= self.idle_budget_mb
    }

    pub fn within_peak(&self) -> bool {
        self.used_mb <= self.peak_budget_mb
    }

    pub fn fraction_of_idle(&self) -> f32 {
        self.used_mb as f32 / self.idle_budget_mb as f32
    }
}

/// Steps clamp for the Image Studio (LCM supports 1-8 steps).
pub fn clamp_steps(steps: f32) -> i32 {
    steps.round().clamp(1.0, 8.0) as i32
}

/// CFG clamp for the Image Studio.
pub fn clamp_cfg(cfg: f32) -> f32 {
    cfg.clamp(1.0, 2.5)
}

/// Process RSS in MB, read from /proc (Linux). None on other platforms.
pub fn process_rss_mb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest.trim().trim_end_matches(" kB").trim().parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// Ensure a writable output directory exists for screenshots.
pub fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    Ok(())
}

pub type SharedTheme = Rc<()>;
