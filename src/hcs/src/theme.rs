//! Theme, keyboard and accessibility preferences for the HCS desktop.
//!
//! Pattern: Omarchy 4's `colors.toml` — ONE file drives the whole system, a
//! template expands it into every app's config, and the swap is atomic. v1
//! persisted `theme.json` per app, so a theme change was neither atomic nor
//! system-wide.
//!
//! The Apple Liquid Glass lesson is encoded too: macOS 27 needed four revisions
//! before translucent controls were legible again. `glass_min_opacity` is
//! therefore never 0 and is asserted by the Gate 10 contrast audit.

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Where the desktop keeps its user-writable preferences.
pub fn state_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(d) = test_override() {
        return d.join("hcs");
    }
    if let Ok(d) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(d).join("hcs")
    } else if let Ok(h) = std::env::var("HOME") {
        PathBuf::from(h).join(".config/hcs")
    } else {
        PathBuf::from("/var/lib/hcs")
    }
}

/// Test-only state directory override.
///
/// The environment is process-global and `cargo test` runs tests on parallel
/// threads, so setting `XDG_CONFIG_HOME` from a test races with every other test
/// in the binary. An override behind a mutex is both isolated and ordered.
///
/// The statics live at module scope in `test_state` because a `static` declared
/// inside each function would be a *different* cell per function: the setter
/// would fill one and the reader would consult another.
///
/// Lock poisoning is ignored so a single failing test cannot cascade into
/// misleading failures everywhere else in the binary.
#[cfg(test)]
mod test_state {
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};

    pub(super) static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    pub(super) static DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

    pub(super) fn current() -> Option<PathBuf> {
        DIR.get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub(super) fn set(dir: Option<PathBuf>) {
        *DIR.get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = dir;
    }
}

#[cfg(test)]
fn test_override() -> Option<PathBuf> {
    test_state::current()
}

#[cfg(test)]
fn with_test_state_dir<T>(tag: &str, f: impl FnOnce(&Path) -> T) -> T {
    use std::sync::MutexGuard;

    let guard: MutexGuard<()> = test_state::LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let dir = std::env::temp_dir().join(format!("hcs-theme-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    test_state::set(Some(dir.clone()));

    let out = f(&dir);

    test_state::set(None);
    drop(guard);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

fn prefs_path() -> PathBuf {
    state_dir().join("prefs.json")
}

/// The canonical colours file. Read-only from the user's perspective: it ships
/// in the image and is the source of truth for every preset.
///
/// The fallback to the repo copy is what lets the test suite exercise the real
/// shipped file rather than a fixture that could drift from it.
pub fn colors_toml() -> PathBuf {
    if let Ok(p) = std::env::var("HCS_THEME_FILE") {
        return PathBuf::from(p);
    }
    let on_device = PathBuf::from("/usr/share/hcs/theme/colors.toml");
    if on_device.exists() {
        on_device
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/includes.chroot/usr/share/hcs/theme/colors.toml")
    }
}

/// Load a preset out of `colors.toml`.
///
/// A deliberately small TOML reader rather than a dependency: the file is ours,
/// its shape is fixed, and a malformed file must produce a clear error rather
/// than a panic in the middle of a theme switch.
pub fn preset(name: &str) -> Result<BTreeMap<String, String>> {
    let path = colors_toml();
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read theme source {}", path.display()))?;

    let header = format!("[presets.{name}]");
    let mut out = BTreeMap::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    if out.is_empty() {
        bail!("theme preset '{name}' is not defined in {}", path.display());
    }
    Ok(out)
}

/// Every preset name defined in the colours file.
pub fn presets() -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(colors_toml()) else {
        // Fall back to the built-in list so the CLI still works on a build host
        // that has no image installed.
        return ["obsidian", "titanium", "stealth", "high_contrast"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    };
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("[presets."))
        .filter_map(|l| l.strip_suffix(']'))
        .map(|s| s.to_string())
        .collect()
}

pub fn describe(name: &str) -> String {
    preset(name)
        .ok()
        .and_then(|p| p.get("description").cloned())
        .unwrap_or_else(|| "(no description)".into())
}

fn prefs() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(txt) = std::fs::read_to_string(prefs_path()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            if let Some(obj) = v.as_object() {
                for (k, val) in obj {
                    if let Some(s) = val.as_str() {
                        out.insert(k.clone(), s.to_string());
                    } else if let Some(b) = val.as_bool() {
                        out.insert(k.clone(), b.to_string());
                    }
                }
            }
        }
    }
    out
}

fn save_prefs(p: &BTreeMap<String, String>) -> anyhow::Result<()> {
    let path = prefs_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json: serde_json::Map<String, serde_json::Value> = p
        .iter()
        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
        .collect();
    let body = serde_json::to_string_pretty(&json)?;
    // Atomic: write to a sibling then rename, so a crash mid-write cannot leave
    // a half-written preferences file that bricks the desktop's appearance.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{body}\n"))?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn active() -> String {
    prefs()
        .get("theme")
        .cloned()
        .unwrap_or_else(|| "obsidian".into())
}

/// Activate a preset: expand the palette for the shell and the GUI apps, then
/// notify them.
///
/// The atomic-swap contract from the v2 plan (§4.3): validate first, write
/// second, reload third. A preset that does not parse must never leave the
/// desktop half-themed.
pub fn set(name: &str) -> anyhow::Result<()> {
    let p = preset(name)?;
    let wallpaper = p.get("wallpaper").cloned().unwrap_or_default();
    let opacity = p
        .get("glass_min_opacity")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.90);
    if opacity <= 0.0 {
        bail!(
            "theme '{name}' declares glass_min_opacity {opacity}; a transparent \
             control surface is unreadable (this is the Liquid Glass lesson)."
        );
    }

    let mut state = prefs();
    state.insert("theme".into(), name.to_string());
    // Remember which *brand* theme was chosen so switching accessibility
    // contrast off can return to it instead of falling back to Obsidian.
    if name != "high_contrast" {
        state.insert("brand_theme".into(), name.to_string());
    }
    save_prefs(&state)?;

    // Publish the resolved palette where the shell and every GUI app read it.
    let out = state_dir().join("theme");
    std::fs::create_dir_all(&out)?;
    let palette: BTreeMap<&str, &str> = p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let json = serde_json::json!({ "preset": name, "values": palette });
    write_atomic(
        &out.join("palette.json"),
        &serde_json::to_string_pretty(&json)?,
    )?;

    // The GUI apps read palette.json; they no longer own a theme.json of their
    // own, so a switch cannot half-apply.
    write_atomic(&out.join("wallpaper"), &wallpaper)?;

    reload_shell();
    Ok(())
}

/// Apply a preset without persisting it (live preview).
pub fn preview(name: &str) -> anyhow::Result<()> {
    let p = preset(name)?;
    let out = state_dir().join("theme");
    std::fs::create_dir_all(&out)?;
    let json = serde_json::json!({ "preset": name, "preview": true, "values": p });
    write_atomic(
        &out.join("palette.json"),
        &serde_json::to_string_pretty(&json)?,
    )?;
    reload_shell();
    Ok(())
}

fn write_atomic(path: &Path, body: &str) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn reload_shell() {
    // Best effort: on a build host there is no compositor, and failing the theme
    // switch because a shell process could not be signalled would be wrong.
    let _ = std::process::Command::new("pkill")
        .args(["-USR2", "quickshell"])
        .status();
}

// --- keyboard ---------------------------------------------------------------

/// Layouts offered by the registry shipped in the image. QWERTZ first: that is
/// the project's home audience, and en-US is one shortcut away.
pub fn keyboard_layouts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("de", "Deutsch (QWERTZ)"),
        ("us", "English (QWERTY, US)"),
        ("fr", "Francais (AZERTY)"),
        ("es", "Espanol (QWERTY)"),
        ("it", "Italiano (QWERTY)"),
        ("gb", "English (QWERTY, UK)"),
    ]
}

pub fn active_keyboard() -> String {
    prefs()
        .get("keyboard")
        .cloned()
        .or_else(|| {
            std::fs::read_to_string("/etc/hcs/keyboard.conf")
                .ok()
                .and_then(|c| {
                    c.lines().find_map(|l| {
                        l.strip_prefix("HCS_XKB_LAYOUT=")
                            .map(|v| v.trim_matches('"').to_string())
                    })
                })
        })
        .unwrap_or_else(|| "de".into())
}

/// Switch layout. The HCS key is never affected: it is a modifier, not a
/// printable key, so no layout can move it.
pub fn set_keyboard(code: &str) -> anyhow::Result<()> {
    if !keyboard_layouts().iter().any(|(c, _)| *c == code) {
        bail!(
            "unknown keyboard layout '{code}'. Available: {}",
            keyboard_layouts()
                .iter()
                .map(|(c, _)| *c)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let mut state = prefs();
    state.insert("keyboard".into(), code.to_string());
    save_prefs(&state)?;

    // Apply immediately for the current session as well, so the user sees the
    // switch rather than having to log out.
    let variant = match code {
        "fr" => ",oss",
        _ => "",
    };
    if Path::new("/etc/hcs/keyboard.conf").exists() {
        std::fs::write(
            "/etc/hcs/keyboard.conf",
            format!(
                "HCS_XKB_LAYOUT=\"{code}\"\nHCS_XKB_VARIANT=\"{}\"\n",
                variant.trim_start_matches(',')
            ),
        )?;
    }
    let _ = std::process::Command::new("setxkbmap")
        .args(["-layout", &format!("{code}{variant}")])
        .status();
    Ok(())
}

pub fn locales() -> Vec<&'static str> {
    vec!["de", "en", "fr", "es", "it"]
}

pub fn active_locale() -> String {
    prefs()
        .get("locale")
        .cloned()
        .or_else(|| {
            std::fs::read_to_string("/etc/hcs/locale")
                .ok()
                .map(|c| c.trim().to_string())
        })
        .unwrap_or_else(|| "de".into())
}

/// The interface language is a separate choice from the keyboard layout: a
/// Swiss user wants a German UI on a French keyboard.
pub fn set_locale(code: &str) -> anyhow::Result<()> {
    if !locales().contains(&code) {
        bail!(
            "unsupported locale '{code}'. Available: {}",
            locales().join(", ")
        );
    }
    let mut state = prefs();
    state.insert("locale".into(), code.to_string());
    save_prefs(&state)?;
    if Path::new("/etc/hcs/locale").exists() {
        std::fs::write("/etc/hcs/locale", format!("{code}\n"))?;
    }
    Ok(())
}

// --- accessibility ----------------------------------------------------------

fn flag(name: &str) -> bool {
    prefs().get(name).map(|v| v == "true").unwrap_or(false)
}

pub fn reduce_motion() -> bool {
    flag("reduce_motion")
}

pub fn set_reduce_motion(on: bool) -> anyhow::Result<()> {
    let mut state = prefs();
    state.insert("reduce_motion".into(), on.to_string());
    save_prefs(&state)?;
    reload_shell();
    Ok(())
}

pub fn high_contrast() -> bool {
    flag("high_contrast")
}

/// Toggling high contrast goes through the theme system rather than setting a
/// flag beside it: the preset *is* the accessibility theme, so there is one
/// code path and no way to end up half-applied.
pub fn set_high_contrast(on: bool) -> anyhow::Result<()> {
    if on {
        set("high_contrast")?;
    } else {
        set(&active_preset_before_contrast())?;
    }
    Ok(())
}

/// When high contrast is on, `active()` reports it, so switching it off needs to
/// know what came before. Preference order: the last explicitly chosen brand
/// theme, else Obsidian.
fn active_preset_before_contrast() -> String {
    prefs()
        .get("brand_theme")
        .cloned()
        .unwrap_or_else(|| "obsidian".into())
}

/// Every preference in one place, for `hcs settings show` and for the agent.
#[allow(dead_code)]
pub fn snapshot() -> BTreeMap<String, String> {
    let mut p = prefs();
    p.entry("theme".into()).or_insert_with(active);
    p.entry("keyboard".into()).or_insert_with(active_keyboard);
    p.entry("locale".into()).or_insert_with(active_locale);
    p.entry("scale".into())
        .or_insert_with(|| scale_percent().to_string());
    p
}

pub fn scale_percent() -> u16 {
    prefs()
        .get("scale")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
}

/// Exact fractional scaling (GNOME 49 lesson: rounded factors blur text).
pub fn set_scale(percent: u16) -> anyhow::Result<()> {
    let allowed = [100u16, 125, 150, 175, 200];
    if !allowed.contains(&percent) {
        bail!(
            "scale {percent}% is not one of the supported steps: {}",
            allowed
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let mut state = prefs();
    state.insert("scale".into(), percent.to_string());
    save_prefs(&state)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every test that writes preferences runs inside `with_test_state_dir`,
    // which serialises them and gives each an isolated directory. Mutating the
    // process environment from a test would race with the other tests in this
    // binary, which cargo runs in parallel.

    #[test]
    fn every_preset_declares_a_glass_floor() {
        // The Liquid Glass lesson, enforced: no preset may be fully transparent.
        for name in presets() {
            let p = preset(&name).expect("preset must parse");
            let opacity: f32 = p
                .get("glass_min_opacity")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0);
            assert!(
                opacity > 0.5,
                "{name} declares glass_min_opacity {opacity}; controls would be unreadable"
            );
        }
    }

    #[test]
    fn every_preset_declares_text_and_wallpaper() {
        for name in presets() {
            let p = preset(&name).expect("preset must parse");
            assert!(p.contains_key("text"), "{name} has no text colour");
            assert!(p.contains_key("bg"), "{name} has no background");
            assert!(p.contains_key("wallpaper"), "{name} has no wallpaper");
        }
    }

    #[test]
    fn presets_are_discovered_from_the_shipped_file() {
        // The shipped colours.toml must define the accessibility preset too, or
        // `hcs theme list` would show it and `hcs theme set` would fail.
        let all = presets();
        for expected in ["obsidian", "titanium", "stealth", "high_contrast"] {
            assert!(
                all.iter().any(|p| p == expected),
                "colors.toml is missing the '{expected}' preset"
            );
        }
    }

    #[test]
    fn unknown_preset_is_an_error() {
        assert!(preset("neon-blaze").is_err());
    }

    #[test]
    fn theme_set_publishes_the_palette_atomically() {
        with_test_state_dir("set", |_| {
            set("stealth").unwrap();
            assert_eq!(active(), "stealth");
            let palette = state_dir().join("theme").join("palette.json");
            assert!(
                palette.exists(),
                "the palette must be published for the shell"
            );
            let txt = std::fs::read_to_string(&palette).unwrap();
            let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
            assert_eq!(v["preset"], "stealth");
            // No temp file may survive a successful write.
            assert!(!palette.with_extension("tmp").exists());
        });
    }

    #[test]
    fn high_contrast_wins_over_a_brand_theme() {
        with_test_state_dir("contrast", |_| {
            set("obsidian").unwrap();
            set_high_contrast(true).unwrap();
            assert_eq!(active(), "high_contrast");
            set_high_contrast(false).unwrap();
            assert_eq!(active(), "obsidian");
        });
    }

    #[test]
    fn switching_theme_records_the_brand_theme() {
        with_test_state_dir("brand", |_| {
            set("titanium").unwrap();
            set_high_contrast(true).unwrap();
            assert_eq!(active(), "high_contrast");
            set_high_contrast(false).unwrap();
            assert_eq!(
                active(),
                "titanium",
                "turning contrast off must return to the theme the user chose"
            );
        });
    }

    #[test]
    fn preview_does_not_persist() {
        with_test_state_dir("preview", |_| {
            set("obsidian").unwrap();
            preview("stealth").unwrap();
            let palette = state_dir().join("theme").join("palette.json");
            let v: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&palette).unwrap()).unwrap();
            assert_eq!(v["preview"], true);
            assert_eq!(active(), "obsidian", "a preview must not persist");
        });
    }

    #[test]
    fn every_shipped_layout_is_offered() {
        let codes: Vec<&str> = keyboard_layouts().iter().map(|(c, _)| *c).collect();
        for expected in ["de", "us", "fr", "es"] {
            assert!(codes.contains(&expected), "{expected} must be offered");
        }
        assert_eq!(codes.first(), Some(&"de"), "QWERTZ is the default layout");
    }

    #[test]
    fn unknown_layout_is_rejected_with_the_available_list() {
        let err = set_keyboard("kl").unwrap_err().to_string();
        assert!(err.contains("de"));
        assert!(err.contains("unknown keyboard layout"));
    }

    #[test]
    fn layout_and_locale_are_independent() {
        with_test_state_dir("locale", |_| {
            set_keyboard("fr").unwrap();
            set_locale("de").unwrap();
            assert_eq!(active_keyboard(), "fr");
            assert_eq!(active_locale(), "de");
        });
    }

    #[test]
    fn unsupported_locale_is_rejected() {
        assert!(set_locale("kl").is_err());
    }

    #[test]
    fn scale_rejects_unsupported_steps() {
        // 130% is not a supported step: exact fractional scaling is a discrete
        // list, and a fuzzy step reintroduces the blurry text it was meant to fix.
        with_test_state_dir("scale", |_| {
            assert!(set_scale(130).is_err());
            assert!(set_scale(150).is_ok());
            assert_eq!(scale_percent(), 150);
        });
    }

    #[test]
    fn reduce_motion_persists() {
        with_test_state_dir("motion", |_| {
            set_reduce_motion(true).unwrap();
            assert!(reduce_motion());
            set_reduce_motion(false).unwrap();
            assert!(!reduce_motion());
        });
    }

    #[test]
    fn snapshot_reports_every_preference() {
        with_test_state_dir("snapshot", |_| {
            let s = snapshot();
            for key in ["theme", "keyboard", "locale", "scale"] {
                assert!(s.contains_key(key), "snapshot must include {key}");
            }
        });
    }
}
