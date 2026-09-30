//! `hcs-settings` — system settings GUI (P3).
//!
//! Theme, wallpaper, privacy defaults, keyboard shortcuts and the mandatory
//! third-party/license disclosure.

use hcs_ui::ThemePreset;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let theme = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| ThemePreset::from_str_opt(s))
        .unwrap_or_else(|| hcs_ui::load_theme());

    hcs_ui::settings::run(theme)
}
