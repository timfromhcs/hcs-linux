//! Settings GUI (P3, v1.3.0): theme, wallpaper, privacy defaults, shortcuts
//! and the mandatory third-party/license disclosure.

use crate::ThemePreset;
use slint::ComponentHandle;

slint::include_modules!();

crate::theme_target!(SettingsWindow);

/// Layout code -> the label the window shows, e.g. `de` -> `de (QWERTZ)`.
///
/// The same registry `hcs settings keyboard` lists. It lives in the GUI crate
/// as well because the Settings window has to render the labels itself; the
/// authority for *which* layout is active stays in the prefs file and
/// `/etc/hcs/keyboard.conf`.
pub const KEYBOARD_LABELS: &[(&str, &str)] = &[
    ("de", "de (QWERTZ)"),
    ("us", "us (QWERTY)"),
    ("fr", "fr (AZERTY)"),
    ("es", "es (QWERTY)"),
    ("it", "it (QWERTY)"),
    ("gb", "gb (QWERTY)"),
];

/// Interface locale -> its own name, written the way a speaker of it writes it.
pub const LOCALE_LABELS: &[(&str, &str)] = &[
    ("en", "English"),
    ("de", "Deutsch"),
    ("fr", "Francais"),
    ("es", "Espanol"),
    ("it", "Italiano"),
];

fn config_home() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(dir).join("hcs");
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home).join(".config/hcs");
    }
    std::path::PathBuf::from("/var/lib/hcs")
}

/// A single string value out of `prefs.json`, if it is there.
fn pref(key: &str) -> Option<String> {
    let txt = std::fs::read_to_string(config_home().join("prefs.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&txt).ok()?;
    v.get(key)?.as_str().map(str::to_string)
}

pub fn active_keyboard_label() -> String {
    let code = pref("keyboard")
        .or_else(|| {
            let c = std::fs::read_to_string("/etc/hcs/keyboard.conf").ok()?;
            c.lines().find_map(|l| {
                l.strip_prefix("HCS_XKB_LAYOUT=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        // QWERTZ is the default, not an accident of the build host.
        .unwrap_or_else(|| "de".into());
    KEYBOARD_LABELS
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(code.clone(), |(_, l)| (*l).to_string())
}

pub fn active_locale_label() -> String {
    let code = pref("locale")
        .or_else(|| {
            std::fs::read_to_string("/etc/hcs/locale")
                .ok()
                .map(|c| c.trim().to_string())
        })
        .unwrap_or_else(|| "de".into());
    LOCALE_LABELS
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(code.clone(), |(_, l)| (*l).to_string())
}

/// The wallpaper suite shipped in `assets/wallpapers`.
pub const WALLPAPERS: &[(&str, &str)] = &[
    (
        "neural_glass_dark.png",
        "Obsidian Neural Gradient (default)",
    ),
    ("frosted_titanium.png", "Frosted Titanium"),
    (
        "cybernetic_stealth.png",
        "Cybernetic Stealth (Security Lab)",
    ),
];

/// Keyboard shortcuts, mirroring `/usr/share/hcs/docs/cheatsheet.json` and
/// `src/hcs-shell/config.kdl`. The HCS key is where the Windows key sits.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("HCS", "Start Menu"),
    ("HCS+Return", "Start Menu"),
    ("HCS+Space", "Cycle Keyboard Layout"),
    ("HCS+/", "Cheatsheet"),
    ("HCS+Q", "Close Window"),
    ("Alt+Tab", "Window Switcher"),
    ("HCS+Left/Right", "Focus Window"),
    ("HCS+K", "HCS Chat"),
    ("HCS+M", "Monitor"),
    ("HCS+I", "Image Studio"),
    ("HCS+T", "Terminal"),
    ("HCS+F", "File Manager"),
    ("HCS+E", "Snap Layouts"),
    ("HCS+Tab", "Task View"),
    ("HCS+Alt+T", "Tor Killswitch"),
    ("HCS+V", "Redacted Clipboard"),
];

/// Third-party components, including the Slint license disclosure that the
/// Royalty-free License 2.0 requires.
pub const THIRD_PARTY: &[&str] = &[
    "Slint 1.18.1 - Royalty-free License 2.0",
    "Debian 13 Trixie base",
    "llama.cpp (MIT)",
    "Niri (GPL-3.0), Quickshell (LGPL-3.0)",
    "Calamares (GPL-3.0), Qwen models",
];

pub fn build_window() -> Result<SettingsWindow, slint::PlatformError> {
    build_window_themed(ThemePreset::Obsidian)
}

/// Build the window with a theme already applied.
pub fn build_window_themed(theme: ThemePreset) -> Result<SettingsWindow, slint::PlatformError> {
    let w = SettingsWindow::new()?;
    crate::apply_hcs_theme!(w, theme);
    // The window shows the theme it was *built* with. Reading the persisted
    // theme here instead meant the Theme card said "obsidian" while the palette
    // was high contrast, and the highlighted button pointed at a theme that was
    // not on screen.
    w.set_selected_theme(theme.as_str().into());
    w.set_selected_wallpaper(WALLPAPERS[0].0.into());
    w.set_keyboard_layout(active_keyboard_label().into());
    w.set_interface_language(active_locale_label().into());
    w.set_shortcuts(
        SHORTCUTS
            .iter()
            .map(|(k, a)| format!("{k:<14} {a}"))
            .collect::<Vec<_>>()
            .join("\n")
            .into(),
    );
    w.set_third_party(THIRD_PARTY.join("\n").into());

    let weak = w.as_weak();
    w.on_pick_theme(move |name| {
        if let Some(w) = weak.upgrade() {
            w.set_selected_theme(name.to_string().into());
            // Live preview: switching the window's palette immediately.
            if let Some(preset) = ThemePreset::from_str_opt(&name) {
                crate::apply_hcs_theme!(w, preset);
            }
        }
    });

    let weak = w.as_weak();
    w.on_pick_wallpaper(move |name| {
        if let Some(w) = weak.upgrade() {
            w.set_selected_wallpaper(name.to_string().into());
        }
    });

    let weak = w.as_weak();
    w.on_pick_keyboard_layout(move |code| {
        if let Some(w) = weak.upgrade() {
            // "cycle" is what the More button sends: the same action as
            // HCS+Space, expressed as a preference. The switch itself lives in
            // the CLI, which owns prefs.json, so the window only reflects it.
            let code = if code == "cycle" {
                next_keyboard(&active_keyboard_label())
            } else {
                code.to_string()
            };
            w.set_keyboard_layout(
                KEYBOARD_LABELS
                    .iter()
                    .find(|(c, _)| *c == code)
                    .map_or(code.clone(), |(_, l)| (*l).to_string())
                    .into(),
            );
        }
    });

    let weak = w.as_weak();
    w.on_pick_interface_language(move |code| {
        if let Some(w) = weak.upgrade() {
            let code = if code == "cycle" {
                next_locale(&active_locale_label())
            } else {
                code.to_string()
            };
            w.set_interface_language(
                LOCALE_LABELS
                    .iter()
                    .find(|(c, _)| *c == code)
                    .map_or(code.clone(), |(_, l)| (*l).to_string())
                    .into(),
            );
        }
    });

    let weak = w.as_weak();
    w.on_save(move || {
        if let Some(w) = weak.upgrade() {
            let name = w.get_selected_theme().to_string();
            let log = match ThemePreset::from_str_opt(&name) {
                Some(p) => match crate::save_theme(p) {
                    Ok(()) => format!("Theme '{name}' saved to {}", config_display()),
                    Err(e) => format!("Could not save theme: {e}"),
                },
                None => format!("Unknown theme '{name}'"),
            };
            w.set_save_log(log.into());
        }
    });

    let weak = w.as_weak();
    w.on_about(move || {
        if let Some(w) = weak.upgrade() {
            w.set_save_log(crate::disclosure().to_string().into());
        }
    });

    Ok(w)
}

fn config_display() -> String {
    crate::theme_config_path().display().to_string()
}

/// The layout after the current one, wrapping around. Used by the More button
/// so the window can step through the registry without owning the persistence.
fn next_keyboard(current_label: &str) -> String {
    let pos = KEYBOARD_LABELS
        .iter()
        .position(|(_, l)| *l == current_label)
        .map(|i| (i + 1) % KEYBOARD_LABELS.len())
        .unwrap_or(0);
    KEYBOARD_LABELS[pos].0.to_string()
}

fn next_locale(current_label: &str) -> String {
    let pos = LOCALE_LABELS
        .iter()
        .position(|(_, l)| *l == current_label)
        .map(|i| (i + 1) % LOCALE_LABELS.len())
        .unwrap_or(0);
    LOCALE_LABELS[pos].0.to_string()
}

pub fn run(theme: ThemePreset) -> anyhow::Result<()> {
    let w = build_window()?;
    crate::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}
