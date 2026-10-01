//! `hcs-docs` — native offline documentation viewer (P3).
//!
//! Replaces the shell-out `hcs-docs` script: renders the manuals in-process
//! with no browser and no network.

use hcs_ui::ThemePreset;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let theme = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| ThemePreset::from_str_opt(s))
        .unwrap_or_else(hcs_ui::load_theme);

    hcs_ui::docs::run(theme)
}
