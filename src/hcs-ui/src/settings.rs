//! Settings GUI (P3, v1.3.0): theme, wallpaper, privacy defaults, shortcuts
//! and the mandatory third-party/license disclosure.

use crate::ThemePreset;
use slint::ComponentHandle;

slint::include_modules!();

crate::theme_target!(SettingsWindow);

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

/// Keyboard shortcuts, mirroring `/usr/share/hcs/docs/cheatsheet.json`.
pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Super", "Start Menu"),
    ("Super+D", "Show Desktop"),
    ("Super+Q", "Close Window"),
    ("Alt+Tab", "Window Switcher"),
    ("Super+Return", "HCS Chat"),
    ("Super+Space", "Search"),
    ("Super+M", "Monitor"),
    ("Super+I", "Image Studio"),
    ("Super+T", "Terminal"),
    ("Super+Alt+T", "Tor Killswitch"),
    ("Super+V", "Redacted Clipboard"),
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
    let current = crate::load_theme();
    w.set_selected_theme(current.as_str().into());
    w.set_selected_wallpaper(WALLPAPERS[0].0.into());
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

pub fn run(theme: ThemePreset) -> anyhow::Result<()> {
    let w = build_window()?;
    crate::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}
