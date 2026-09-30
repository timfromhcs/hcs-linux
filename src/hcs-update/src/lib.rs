//! HCS Update — the level-based update manager with snapshots and rollback.
//!
//! Gap closed from the v2 audit (B-12). Two reference systems contribute:
//!   - **Linux Mint's Update Manager**: levels 1–5, from "safe and tested" to
//!     "risky or experimental", each of which can be applied automatically or
//!     held for review. This is the most honest update model any distro ships,
//!     because it makes risk *visible* instead of all-or-nothing.
//!   - **Omarchy's btrfs snapshots**: a snapshot before every update and a
//!     rollback entry in the boot menu, so a bad update is never a catastrophe.
//!
//! The rollback decision logic is pure so it can be tested exhaustively without
//! a btrfs volume.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Risk level of a single package update. Mirrors Mint's levels so the mental
/// model transfers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Security updates only. Auto-applied unless the user says otherwise.
    Security,
    /// Bug fixes for already-installed packages.
    Stable,
    /// New upstream point releases of installed packages.
    Minor,
    /// New packages offered by the distribution.
    New,
    /// Backports and experimental builds. Manual only.
    Experimental,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Security => "security",
            Level::Stable => "stable",
            Level::Minor => "minor",
            Level::New => "new",
            Level::Experimental => "experimental",
        }
    }

    pub fn all() -> &'static [Level] {
        &[
            Level::Security,
            Level::Stable,
            Level::Minor,
            Level::New,
            Level::Experimental,
        ]
    }

    /// Level 1 is safe; level 5 is not. Used by the UI to colour the list.
    pub fn number(self) -> u8 {
        match self {
            Level::Security => 1,
            Level::Stable => 2,
            Level::Minor => 3,
            Level::New => 4,
            Level::Experimental => 5,
        }
    }

    /// Experimental updates are never applied automatically, regardless of what
    /// the auto-apply set says.
    pub fn auto_applyable(self) -> bool {
        !matches!(self, Level::Experimental)
    }

    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Level::Security),
            2 => Some(Level::Stable),
            3 => Some(Level::Minor),
            4 => Some(Level::New),
            5 => Some(Level::Experimental),
            _ => None,
        }
    }
}

/// One pending update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Update {
    pub package: String,
    pub from: String,
    pub to: String,
    pub level: Level,
}

impl Update {
    pub fn new(package: &str, from: &str, to: &str, level: Level) -> Self {
        Self {
            package: package.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            level,
        }
    }
}

/// Which levels are applied without asking. Everything outside this set is
/// held for review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    auto_levels: Vec<Level>,
}

impl Default for Policy {
    /// Mint's default posture: security and stable go automatically; minor,
    /// new and experimental wait for a human. On a live ISO, `New` in
    /// particular should never be automatic — a surprise package appearing in a
    /// privacy-focused system is a supply-chain decision, not a convenience.
    fn default() -> Self {
        Self {
            auto_levels: vec![Level::Security, Level::Stable],
        }
    }
}

impl Policy {
    pub fn new(auto_levels: Vec<Level>) -> Self {
        Self { auto_levels }
    }

    pub fn auto(&self, level: Level) -> bool {
        level.auto_applyable() && self.auto_levels.contains(&level)
    }

    pub fn with_auto(mut self, level: Level) -> Self {
        if !self.auto_levels.contains(&level) {
            self.auto_levels.push(level);
            self.auto_levels.sort();
        }
        self
    }

    pub fn without_auto(mut self, level: Level) -> Self {
        self.auto_levels.retain(|l| *l != level);
        self
    }

    pub fn auto_levels(&self) -> &[Level] {
        &self.auto_levels
    }
}

/// Split a pending list into what would happen now and what waits for review.
pub fn partition(updates: &[Update], policy: &Policy) -> (Vec<Update>, Vec<Update>) {
    updates.iter().cloned().partition(|u| policy.auto(u.level))
}

/// A snapshot taken before an update, so the update can be undone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    /// Unix seconds.
    pub created: i64,
    /// Snapshot label, e.g. "pre-2.0.1".
    pub label: String,
    /// Total size on disk in MB.
    pub size_mb: u64,
    pub packages_before: u32,
}

/// What a rollback would do, evaluated before anything is touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackPlan {
    /// Nothing to roll back to. The message is explicit so the CLI does not
    /// print a bare "no snapshot found".
    NoSnapshot,
    /// The newest snapshot can be restored; this is its label.
    Restore {
        snapshot: Snapshot,
        packages_to_revert: u32,
    },
    /// Something newer than the given snapshot exists, so rolling back would
    /// discard work. Requires `--force`.
    WouldDiscardNewer { snapshot: Snapshot, newer: String },
}

/// Decide the rollback. Pure, so every branch is directly testable.
pub fn plan_rollback(
    snapshots: &[Snapshot],
    current_packages: u32,
    target: Option<&str>,
    force: bool,
) -> RollbackPlan {
    if snapshots.is_empty() {
        return RollbackPlan::NoSnapshot;
    }
    let mut sorted: Vec<&Snapshot> = snapshots.iter().collect();
    sorted.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.id.cmp(&b.id)));

    let chosen = match target {
        Some(id) => match sorted.iter().find(|s| s.id == id) {
            Some(s) => (*s).clone(),
            None => return RollbackPlan::NoSnapshot,
        },
        None => sorted[0].clone(),
    };

    let reverted = current_packages.saturating_sub(chosen.packages_before);
    let newer = sorted
        .iter()
        .find(|s| s.created > chosen.created)
        .map(|s| s.label.clone());

    match newer {
        Some(label) if !force => RollbackPlan::WouldDiscardNewer {
            snapshot: chosen,
            newer: label,
        },
        _ => RollbackPlan::Restore {
            snapshot: chosen,
            packages_to_revert: reverted,
        },
    }
}

/// Whether the update list requires a reboot. Windows 11 shows this as a tray
/// warning after an update that ships a new kernel; the same surprise on Linux
/// is how people end up running an old kernel for months.
pub fn requires_reboot(current_kernel: &str, pending: &[Update]) -> bool {
    pending
        .iter()
        .any(|u| u.package.starts_with("linux-image") && u.to != current_kernel)
}

/// Human summary for the update window, in the shape Mint uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub by_level: BTreeMap<String, u32>,
    pub auto_count: usize,
    pub held_count: usize,
    pub reboot_required: bool,
}

pub fn summarise(updates: &[Update], policy: &Policy, current_kernel: &str) -> Summary {
    let mut by_level: BTreeMap<String, u32> = BTreeMap::new();
    for u in updates {
        *by_level.entry(u.level.as_str().to_string()).or_insert(0) += 1;
    }
    let (auto, held) = partition(updates, policy);
    Summary {
        by_level,
        auto_count: auto.len(),
        held_count: held.len(),
        reboot_required: requires_reboot(current_kernel, updates),
    }
}

impl fmt::Display for RollbackPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RollbackPlan::NoSnapshot => write!(f, "no snapshot is available to roll back to"),
            RollbackPlan::Restore {
                snapshot,
                packages_to_revert,
            } => write!(
                f,
                "restore snapshot '{}' ({} MB, {} package(s) to revert)",
                snapshot.label, snapshot.size_mb, packages_to_revert
            ),
            RollbackPlan::WouldDiscardNewer { snapshot, newer } => write!(
                f,
                "snapshot '{}' is older than '{}'; rolling back discards the newer one (use --force)",
                snapshot.label, newer
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn updates() -> Vec<Update> {
        vec![
            Update::new("linux-image-amd64", "6.11", "6.12", Level::Stable),
            Update::new("openssl", "3.0.1", "3.0.2", Level::Security),
            Update::new("vlc", "3.0.18", "3.0.20", Level::Minor),
            Update::new("firefox-esr", "1", "2", Level::New),
            Update::new("experimental-thing", "0", "1", Level::Experimental),
        ]
    }

    fn snapshot(id: &str, created: i64, label: &str, pkgs: u32) -> Snapshot {
        Snapshot {
            id: id.into(),
            created,
            label: label.into(),
            size_mb: 512,
            packages_before: pkgs,
        }
    }

    #[test]
    fn levels_order_from_safe_to_risky() {
        assert!(Level::Security < Level::Experimental);
        assert_eq!(Level::all().len(), 5);
    }

    #[test]
    fn level_numbers_round_trip() {
        for l in Level::all() {
            assert_eq!(Level::from_number(l.number()), Some(*l));
        }
        assert_eq!(Level::from_number(0), None);
        assert_eq!(Level::from_number(9), None);
    }

    #[test]
    fn experimental_is_never_auto_applyable() {
        assert!(!Level::Experimental.auto_applyable());
        for l in Level::all().iter().filter(|l| **l != Level::Experimental) {
            assert!(
                l.auto_applyable(),
                "{} should be auto-applyable",
                l.as_str()
            );
        }
    }

    #[test]
    fn default_policy_auto_applies_only_security_and_stable() {
        let p = Policy::default();
        assert!(p.auto(Level::Security));
        assert!(p.auto(Level::Stable));
        assert!(!p.auto(Level::Minor));
        assert!(!p.auto(Level::New));
        assert!(!p.auto(Level::Experimental));
    }

    #[test]
    fn experimental_stays_manual_even_when_the_policy_lists_it() {
        let p = Policy::new(vec![Level::Experimental]);
        assert!(
            !p.auto(Level::Experimental),
            "policy must not be able to auto-apply experimental updates"
        );
    }

    #[test]
    fn partition_splits_auto_from_held() {
        let (auto, held) = partition(&updates(), &Policy::default());
        assert_eq!(auto.len(), 2);
        assert_eq!(held.len(), 3);
        assert!(auto.iter().all(|u| u.level != Level::Experimental));
    }

    #[test]
    fn policy_can_be_widened() {
        let p = Policy::default().with_auto(Level::Minor);
        assert!(p.auto(Level::Minor));
        let (auto, _) = partition(&updates(), &p);
        assert_eq!(auto.len(), 3);
    }

    #[test]
    fn policy_can_be_narrowed() {
        let p = Policy::default().without_auto(Level::Security);
        assert!(!p.auto(Level::Security));
    }

    #[test]
    fn adding_the_same_level_twice_is_idempotent() {
        let p = Policy::default()
            .with_auto(Level::Stable)
            .with_auto(Level::Stable);
        assert_eq!(
            p.auto_levels()
                .iter()
                .filter(|l| **l == Level::Stable)
                .count(),
            1
        );
    }

    #[test]
    fn summary_counts_per_level() {
        let s = summarise(&updates(), &Policy::default(), "6.11");
        assert_eq!(s.by_level.get("security"), Some(&1));
        assert_eq!(s.by_level.get("stable"), Some(&1));
        assert_eq!(s.by_level.get("minor"), Some(&1));
        assert_eq!(s.by_level.get("new"), Some(&1));
        assert_eq!(s.by_level.get("experimental"), Some(&1));
        assert_eq!(s.auto_count, 2);
        assert_eq!(s.held_count, 3);
    }

    #[test]
    fn kernel_change_flags_reboot() {
        let s = summarise(&updates(), &Policy::default(), "6.11");
        assert!(s.reboot_required);
    }

    #[test]
    fn same_kernel_does_not_flag_reboot() {
        let s = summarise(&updates(), &Policy::default(), "6.12");
        assert!(!s.reboot_required);
    }

    #[test]
    fn no_snapshots_means_no_rollback() {
        assert_eq!(
            plan_rollback(&[], 10, None, false),
            RollbackPlan::NoSnapshot
        );
    }

    #[test]
    fn newest_snapshot_is_chosen_by_default() {
        let snaps = vec![
            snapshot("a", 100, "pre-2.0.0", 5),
            snapshot("b", 200, "pre-2.0.1", 7),
        ];
        match plan_rollback(&snaps, 9, None, false) {
            RollbackPlan::Restore {
                snapshot,
                packages_to_revert,
            } => {
                assert_eq!(snapshot.id, "b");
                assert_eq!(packages_to_revert, 2);
            }
            other => panic!("expected Restore, got {other:?}"),
        }
    }

    #[test]
    fn specific_snapshot_can_be_requested() {
        let snaps = vec![
            snapshot("a", 100, "pre-2.0.0", 5),
            snapshot("b", 200, "pre-2.0.1", 7),
        ];
        match plan_rollback(&snaps, 9, Some("a"), false) {
            RollbackPlan::WouldDiscardNewer { snapshot, newer } => {
                assert_eq!(snapshot.id, "a");
                assert_eq!(newer, "pre-2.0.1");
            }
            other => panic!("expected WouldDiscardNewer, got {other:?}"),
        }
    }

    #[test]
    fn force_overrides_the_newer_snapshot_warning() {
        let snaps = vec![
            snapshot("a", 100, "pre-2.0.0", 5),
            snapshot("b", 200, "pre-2.0.1", 7),
        ];
        match plan_rollback(&snaps, 9, Some("a"), true) {
            RollbackPlan::Restore { snapshot, .. } => assert_eq!(snapshot.id, "a"),
            other => panic!("expected Restore with force, got {other:?}"),
        }
    }

    #[test]
    fn unknown_snapshot_id_yields_no_snapshot() {
        let snaps = vec![snapshot("a", 100, "pre-2.0.0", 5)];
        assert_eq!(
            plan_rollback(&snaps, 9, Some("nope"), false),
            RollbackPlan::NoSnapshot
        );
    }

    #[test]
    fn reverting_never_underflows() {
        let snaps = vec![snapshot("a", 100, "pre", 20)];
        match plan_rollback(&snaps, 5, None, false) {
            RollbackPlan::Restore {
                packages_to_revert, ..
            } => assert_eq!(packages_to_revert, 0),
            other => panic!("expected Restore, got {other:?}"),
        }
    }

    #[test]
    fn rollback_display_mentions_force_when_discarding() {
        let p = RollbackPlan::WouldDiscardNewer {
            snapshot: snapshot("a", 1, "old", 1),
            newer: "new".into(),
        };
        assert!(p.to_string().contains("--force"));
    }

    #[test]
    fn no_snapshot_display_is_explicit() {
        assert!(RollbackPlan::NoSnapshot.to_string().contains("no snapshot"));
    }
}
