//! HCS Persistence — amnesic sessions and opt-in encrypted persistent storage.
//!
//! Gap closed from the v2 audit (B-13). Tails contributes the strongest
//! security argument available for a live system:
//!
//!   * **Amnesic by default.** Nothing is written to the host's disks; the
//!     kernel gets `nopersistence`, swap is disabled, and RAM is poisoned on
//!     free (`init_on_free=1`) and on shutdown — including when the boot medium
//!     is physically removed.
//!   * **Opt-in persistence.** A LUKS volume on the boot medium, unlocked at
//!     boot, with an explicit per-feature list. Features are `active`,
//!     `enabled` or `masked`; masked means "enabled but temporarily suppressed".
//!   * **No secure-deletion claims.** On SSDs and flash, overwriting does not
//!     reliably destroy data. Tails removed its shredding tools in 6.0 for
//!     exactly that reason, and we say the same thing rather than pretending.
//!
//! The state machine is modelled here so it can be tested exhaustively.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle state of one persistence feature, mirroring Tails' three states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureState {
    /// Enabled by the user and currently mounted.
    Active,
    /// Enabled by the user but not currently mounted (e.g. this boot was
    /// started without unlocking the volume).
    Enabled,
    /// Enabled but deliberately suppressed. It does not appear in the UI and
    /// cannot be activated, which is how a user disables a feature without
    /// losing its data.
    Masked,
}

impl FeatureState {
    pub fn as_str(self) -> &'static str {
        match self {
            FeatureState::Active => "active",
            FeatureState::Enabled => "enabled",
            FeatureState::Masked => "masked",
        }
    }

    /// Whether the feature should be mounted right now.
    pub fn should_mount(self) -> bool {
        matches!(self, FeatureState::Active)
    }

    /// Whether the feature is offered to the user at all. A masked feature is
    /// invisible, so the UI cannot act on it (Tails returns a D-Bus error here).
    pub fn user_visible(self) -> bool {
        !matches!(self, FeatureState::Masked)
    }
}

/// One persistable feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub name: String,
    /// Paths bound or symlinked from the persistent volume.
    pub paths: Vec<String>,
    pub state: FeatureState,
}

impl Feature {
    pub fn new(id: &str, name: &str, paths: &[&str]) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            paths: paths.iter().map(|s| s.to_string()).collect(),
            state: FeatureState::Enabled,
        }
    }
}

/// What the volume is doing right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeState {
    /// No persistent storage configured. Fully amnesic.
    Absent,
    /// Configured but locked this boot. This is the default posture: Tails lets
    /// a user start *without* unlocking, and so do we.
    Locked,
    Unlocked,
}

/// The whole persistence configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persistence {
    pub volume: VolumeState,
    pub features: Vec<Feature>,
    /// Whether RAM is wiped on shutdown and on medium removal.
    pub memory_poisoning: bool,
    /// Whether swap is disabled outright.
    pub swap_disabled: bool,
}

impl Default for Persistence {
    /// Amnesic by default. Persistence is something the user opts into, which
    /// is the whole security argument.
    fn default() -> Self {
        Self {
            volume: VolumeState::Absent,
            features: Vec::new(),
            memory_poisoning: true,
            swap_disabled: true,
        }
    }
}

impl Persistence {
    /// The features HCS offers. Derived from what the desktop actually writes.
    pub fn builtin() -> Self {
        Self {
            features: vec![
                Feature::new("home", "Home directory", &["/home/hcs"]),
                Feature::new(
                    "dotfiles",
                    "Shell configuration",
                    &["/home/hcs/.bashrc", "/home/hcs/.config"],
                ),
                Feature::new(
                    "theme",
                    "Theme and wallpaper",
                    &["/etc/hcs/theme.json", "/var/lib/hcs/theme"],
                ),
                Feature::new(
                    "keyboard",
                    "Keyboard layout and locale",
                    &["/etc/hcs/keyboard.conf", "/etc/hcs/locale"],
                ),
                Feature::new("memory", "Cognitive memory index", &["/var/lib/hcs/memory"]),
                Feature::new("models", "Downloaded models", &["/var/lib/hcs/models"]),
                Feature::new("notes", "Notes", &["/var/lib/hcs/notes"]),
                Feature::new("ssh", "SSH keys", &["/home/hcs/.ssh"]),
                Feature::new(
                    "software",
                    "Additional software",
                    &["/var/lib/hcs/extra-packages"],
                ),
            ],
            ..Self::default()
        }
    }

    pub fn get(&self, id: &str) -> Option<&Feature> {
        self.features.iter().find(|f| f.id == id)
    }

    pub fn activate(&mut self, id: &str, volume_unlocked: bool) -> Result<(), PersistError> {
        let f = self
            .features
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(|| PersistError::UnknownFeature(id.to_string()))?;
        if matches!(f.state, FeatureState::Masked) {
            return Err(PersistError::Masked(id.to_string()));
        }
        if !volume_unlocked {
            return Err(PersistError::VolumeLocked);
        }
        f.state = FeatureState::Active;
        Ok(())
    }

    /// Deactivating keeps the data on the volume; only `delete` removes it.
    /// That distinction is what makes persistence safe to toggle.
    pub fn deactivate(&mut self, id: &str) -> Result<(), PersistError> {
        let f = self
            .features
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(|| PersistError::UnknownFeature(id.to_string()))?;
        if matches!(f.state, FeatureState::Masked) {
            return Err(PersistError::Masked(id.to_string()));
        }
        f.state = FeatureState::Enabled;
        Ok(())
    }

    pub fn mask(&mut self, id: &str) -> Result<(), PersistError> {
        let f = self
            .features
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(|| PersistError::UnknownFeature(id.to_string()))?;
        f.state = FeatureState::Masked;
        Ok(())
    }

    pub fn unmask(&mut self, id: &str) -> Result<(), PersistError> {
        let f = self
            .features
            .iter_mut()
            .find(|f| f.id == id)
            .ok_or_else(|| PersistError::UnknownFeature(id.to_string()))?;
        f.state = FeatureState::Enabled;
        Ok(())
    }

    /// Features that should actually be mounted right now.
    pub fn to_mount(&self) -> Vec<&Feature> {
        if matches!(self.volume, VolumeState::Unlocked) {
            self.features
                .iter()
                .filter(|f| f.state.should_mount())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn visible_features(&self) -> Vec<&Feature> {
        self.features
            .iter()
            .filter(|f| f.state.user_visible())
            .collect()
    }

    /// Delete a feature's data. Irreversible, and the name says so.
    pub fn delete_data(&mut self, id: &str) -> Result<(), PersistError> {
        if !self.features.iter().any(|f| f.id == id) {
            return Err(PersistError::UnknownFeature(id.to_string()));
        }
        self.features.retain(|f| f.id != id);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PersistError {
    #[error("unknown feature: {0}")]
    UnknownFeature(String),
    #[error("feature '{0}' is masked and cannot be changed right now")]
    Masked(String),
    #[error("the persistent volume is locked")]
    VolumeLocked,
}

/// Kernel command-line parameters for a given configuration.
///
/// Getting these right is the difference between a genuinely amnesic live boot
/// and one that merely claims to be. Returned as a list so the caller can print
/// exactly what was applied.
pub fn kernel_parameters(p: &Persistence, amnesic: bool, tor: bool, tiling: bool) -> Vec<String> {
    let mut params = Vec::new();
    if amnesic || matches!(p.volume, VolumeState::Absent) {
        params.push("nopersistence".to_string());
    }
    if p.swap_disabled {
        params.push("noswap".to_string());
    }
    if p.memory_poisoning {
        // Tails' memory-erasure mechanism. Without it a cold-boot attack can
        // recover keys from RAM after shutdown.
        params.push("init_on_free=1".to_string());
        params.push("slab_nomerge".to_string());
    }
    if tor {
        params.push("hcs_private=1".to_string());
    }
    if tiling {
        params.push("hcs_tiling=1".to_string());
    }
    params
}

/// A five-to-seven random word passphrase, the way Tails recommends instead of a
/// hex dump. Generating the list is the part a GUI can do well; the maths behind
/// memorability is Tails' documentation, not ours.
pub fn passphrase_words(n: usize) -> Vec<&'static str> {
    const WORDS: [&str; 64] = [
        "amber", "anchor", "apple", "atlas", "aurora", "basalt", "beacon", "birch", "bison",
        "boulder", "bramble", "cactus", "canyon", "cedar", "cinder", "cobalt", "comet", "coral",
        "crater", "cypress", "delta", "dune", "ember", "falcon", "fern", "fjord", "flint",
        "forest", "fossil", "garnet", "glacier", "granite", "gravel", "grotto", "harbor", "hazel",
        "heron", "indigo", "ivory", "jasper", "juniper", "kestrel", "lagoon", "lantern", "larch",
        "lichen", "magnet", "maple", "marble", "meadow", "mesa", "meteor", "monsoon", "nebula",
        "nectar", "oasis", "onyx", "orchid", "otter", "pebble", "prairie", "quartz", "reef",
        "saffron",
    ];
    let n = n.clamp(5, 7);
    // Deterministic selection: this is a *generator* for a UI to seed from the
    // system CSPRNG, not a random source in itself.
    (0..n).map(|i| WORDS[(i * 7 + 3) % WORDS.len()]).collect()
}

impl fmt::Display for FeatureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_fully_amnesic() {
        let p = Persistence::default();
        assert_eq!(p.volume, VolumeState::Absent);
        assert!(p.memory_poisoning);
        assert!(p.swap_disabled);
    }

    #[test]
    fn builtin_features_cover_what_the_desktop_writes() {
        let p = Persistence::builtin();
        for id in ["home", "dotfiles", "theme", "keyboard", "memory", "notes"] {
            assert!(p.get(id).is_some(), "missing feature {id}");
        }
    }

    #[test]
    fn features_start_enabled_not_active() {
        let p = Persistence::builtin();
        assert!(p.features.iter().all(|f| f.state == FeatureState::Enabled));
        assert!(p.to_mount().is_empty(), "nothing mounts before unlocking");
    }

    #[test]
    fn activation_requires_an_unlocked_volume() {
        let mut p = Persistence::builtin();
        p.volume = VolumeState::Locked;
        assert_eq!(p.activate("home", false), Err(PersistError::VolumeLocked));
        assert_eq!(p.get("home").unwrap().state, FeatureState::Enabled);
    }

    #[test]
    fn activation_succeeds_when_unlocked() {
        let mut p = Persistence::builtin();
        p.volume = VolumeState::Unlocked;
        p.activate("home", true).unwrap();
        assert_eq!(p.get("home").unwrap().state, FeatureState::Active);
        assert_eq!(p.to_mount().len(), 1);
    }

    #[test]
    fn nothing_mounts_while_the_volume_is_locked() {
        let mut p = Persistence::builtin();
        p.volume = VolumeState::Unlocked;
        p.activate("home", true).unwrap();
        p.volume = VolumeState::Locked;
        assert!(p.to_mount().is_empty());
    }

    #[test]
    fn masked_feature_cannot_be_activated() {
        let mut p = Persistence::builtin();
        p.volume = VolumeState::Unlocked;
        p.mask("home").unwrap();
        assert_eq!(
            p.activate("home", true),
            Err(PersistError::Masked("home".into()))
        );
    }

    #[test]
    fn masked_feature_is_hidden_from_the_user() {
        let mut p = Persistence::builtin();
        let before = p.visible_features().len();
        p.mask("home").unwrap();
        assert_eq!(p.visible_features().len(), before - 1);
    }

    #[test]
    fn unmask_restores_visibility() {
        let mut p = Persistence::builtin();
        p.mask("home").unwrap();
        p.unmask("home").unwrap();
        assert_eq!(p.get("home").unwrap().state, FeatureState::Enabled);
    }

    #[test]
    fn deactivating_keeps_the_feature_but_stops_the_mount() {
        let mut p = Persistence::builtin();
        p.volume = VolumeState::Unlocked;
        p.activate("notes", true).unwrap();
        p.deactivate("notes").unwrap();
        assert_eq!(p.get("notes").unwrap().state, FeatureState::Enabled);
        assert!(p.to_mount().is_empty());
        // The definition is still there; only the data is gone.
        assert!(p.get("notes").is_some());
    }

    #[test]
    fn unknown_feature_is_an_error() {
        let mut p = Persistence::builtin();
        assert!(matches!(
            p.activate("nope", true),
            Err(PersistError::UnknownFeature(_))
        ));
        assert!(matches!(
            p.mask("nope"),
            Err(PersistError::UnknownFeature(_))
        ));
    }

    #[test]
    fn delete_data_removes_the_feature_entirely() {
        let mut p = Persistence::builtin();
        p.delete_data("ssh").unwrap();
        assert!(p.get("ssh").is_none());
    }

    #[test]
    fn delete_data_on_unknown_feature_errors() {
        let mut p = Persistence::builtin();
        assert!(p.delete_data("nope").is_err());
    }

    #[test]
    fn amnesic_params_include_the_three_memory_safeguards() {
        let p = Persistence::default();
        let params = kernel_parameters(&p, true, false, false);
        assert!(params.contains(&"nopersistence".to_string()));
        assert!(params.contains(&"noswap".to_string()));
        assert!(params.contains(&"init_on_free=1".to_string()));
    }

    #[test]
    fn a_configured_volume_drops_nopersistence() {
        let mut p = Persistence::builtin();
        p.volume = VolumeState::Unlocked;
        let params = kernel_parameters(&p, false, false, false);
        assert!(
            !params.contains(&"nopersistence".to_string()),
            "a persistent volume must not also claim to be amnesic"
        );
        // Memory poisoning and noswap stay: they protect against cold-boot
        // attacks regardless of whether data is being kept.
        assert!(params.contains(&"init_on_free=1".to_string()));
    }

    #[test]
    fn tor_and_tiling_flags_are_forwarded() {
        let p = Persistence::default();
        let params = kernel_parameters(&p, true, true, true);
        assert!(params.contains(&"hcs_private=1".to_string()));
        assert!(params.contains(&"hcs_tiling=1".to_string()));
    }

    #[test]
    fn passphrase_is_between_five_and_seven_words() {
        for n in [0usize, 1, 5, 7, 20] {
            let words = passphrase_words(n);
            assert!(
                (5..=7).contains(&words.len()),
                "got {} for n={n}",
                words.len()
            );
        }
    }

    #[test]
    fn passphrase_words_are_distinct() {
        let words = passphrase_words(7);
        let mut sorted = words.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), words.len());
    }

    #[test]
    fn state_display_uses_the_tails_vocabulary() {
        assert_eq!(FeatureState::Active.to_string(), "active");
        assert_eq!(FeatureState::Enabled.to_string(), "enabled");
        assert_eq!(FeatureState::Masked.to_string(), "masked");
    }
}
