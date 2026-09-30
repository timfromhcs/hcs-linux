//! Headless render tool for the HCS GUI visual-regression gate.
//!
//! Renders every GUI view of every app with the Slint software renderer (no
//! GPU, no display, no Wayland) and writes PNGs for `scripts/verify_gui.py`.
//! Dev-only: this binary is NOT part of the production ISO.
//!
//! Usage:
//!   hcs-gui-shots --out-dir qa/gui --theme obsidian
//!   hcs-gui-shots --list
//!   hcs-gui-shots --update-references
//!   hcs-gui-shots --hold-seconds 6        (used by the RAM audit)
//!
//! IMPORTANT: each view is *created and rendered immediately*, one at a time.
//! The headless harness resolves the window adapter from the most recently
//! created component, so building all views up front and rendering afterwards
//! would attach every render to the last view's window (all-black PNGs).

use hcs_ui::harness::{render_to_png, RenderError};
use hcs_ui::ThemePreset;
use std::path::PathBuf;

/// The renderable views. Concrete dispatch (no trait objects) because Slint
/// component handles carry associated types and are not object-safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    FoundationGallery,
    Chat,
    /// Image Studio tab of the chat window (tab-index 1).
    ChatImageStudio,
    Monitor,
    Control,
    Search,
    Diagnose,
    Docs,
    Settings,
}

struct Spec {
    kind: Kind,
    name: &'static str,
    width: u32,
    height: u32,
}

const SPECS: &[Spec] = &[
    Spec {
        kind: Kind::FoundationGallery,
        name: "foundation-gallery",
        width: 720,
        height: 560,
    },
    Spec {
        kind: Kind::Chat,
        name: "chat",
        width: 860,
        height: 620,
    },
    Spec {
        kind: Kind::ChatImageStudio,
        name: "chat-image-studio",
        width: 860,
        height: 620,
    },
    Spec {
        kind: Kind::Monitor,
        name: "monitor",
        width: 760,
        height: 560,
    },
    Spec {
        kind: Kind::Control,
        name: "control",
        width: 720,
        height: 520,
    },
    Spec {
        kind: Kind::Search,
        name: "search",
        width: 680,
        height: 480,
    },
    Spec {
        kind: Kind::Diagnose,
        name: "diagnose",
        width: 760,
        height: 540,
    },
    Spec {
        kind: Kind::Docs,
        name: "docs",
        width: 820,
        height: 600,
    },
    Spec {
        kind: Kind::Settings,
        name: "settings",
        // Must match SettingsWindow's own size. When these drifted apart the
        // reference image was a cropped window: the LayoutError branch never
        // fired, so the gate passed while showing half a settings page.
        width: 800,
        height: 700,
    },
];

/// Make the render independent of whatever the machine's config happens to say.
///
/// A component that reads the live preferences (the Settings window reads the
/// active keyboard layout and locale) produces a *different image* on a
/// machine where someone switched to AZERTY. That is fine in production and
/// fatal for a visual-regression reference: the gate would then fail or pass
/// depending on who ran it. So the render points the config at a fixed
/// throwaway directory with the documented defaults in it.
///
/// `settings.json` is written rather than left empty, because an absent
/// `prefs.json` would fall through to the *system* files instead.
fn pin_state_to_defaults() {
    let dir = std::env::temp_dir().join("hcs-gui-shots-state");
    let _ = std::fs::create_dir_all(&dir);
    std::env::set_var("XDG_CONFIG_HOME", &dir);
    let prefs = serde_json::json!({
        "theme": "obsidian",
        "brand_theme": "obsidian",
        "keyboard": "de",
        "locale": "en",
        "reduce_motion": "false",
        "scale": "100",
    });
    if let Ok(body) = serde_json::to_string_pretty(&prefs) {
        let _ = std::fs::write(dir.join("hcs/prefs.json"), body);
    }
    // The Settings window also reads a theme.json; an empty preset list would
    // leave the theme buttons unlabelled.
    let theme = serde_json::json!({ "theme": "obsidian" });
    if let Ok(body) = serde_json::to_string_pretty(&theme) {
        let _ = std::fs::write(dir.join("hcs/theme.json"), body);
    }
}

/// Create and render one view. Each arm builds its component and renders it
/// straight away, which is what the harness requires.
fn render_view(spec: &Spec, theme: ThemePreset, out: &std::path::Path) -> Result<(), RenderError> {
    macro_rules! shot {
        ($c:expr) => {{
            let c = $c.map_err(|e| RenderError::Platform(format!("{e}")))?;
            render_to_png(&c, spec.width, spec.height, out)
        }};
    }
    match spec.kind {
        Kind::FoundationGallery => shot!(hcs_ui::build_gallery_themed(theme)),
        Kind::Chat => shot!(hcs_chat::gui::build_window(
            &hcs_chat::gui::ChatContext::offline("gui-shots"),
            theme
        )),
        Kind::ChatImageStudio => {
            let c = hcs_chat::gui::build_window(
                &hcs_chat::gui::ChatContext::offline("gui-shots"),
                theme,
            )
            .map_err(|e| RenderError::Platform(format!("{e}")))?;
            c.set_tab_index(1);
            render_to_png(&c, spec.width, spec.height, out)
        }
        Kind::Monitor => shot!(hcs_monitor::gui::build_window_themed(theme)),
        Kind::Control => shot!(hcs_control::gui::build_window_themed(theme)),
        Kind::Search => shot!(hcs_search::gui::build_window_themed(theme)),
        Kind::Diagnose => shot!(hcs_diagnose::gui::build_window_themed(theme)),
        Kind::Docs => shot!(hcs_ui::docs::build_window_themed(theme)),
        Kind::Settings => shot!(hcs_ui::settings::build_window_themed(theme)),
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--list") {
        for s in SPECS {
            println!("{}\t{}x{}", s.name, s.width, s.height);
        }
        return Ok(());
    }

    let mut out_dir = PathBuf::from("qa/gui");
    let mut theme = ThemePreset::Obsidian;
    let mut only: Option<String> = None;
    let mut update_refs = false;
    let mut hold: f64 = 0.0;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--out-dir" => {
                out_dir = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            "--theme" => {
                let name = args.get(i + 1).cloned().unwrap_or_default();
                theme = ThemePreset::from_str_opt(&name)
                    .ok_or_else(|| anyhow::anyhow!("unknown theme: {name}"))?;
                i += 2;
            }
            "--view" => {
                only = args.get(i + 1).cloned();
                i += 2;
            }
            "--hold-seconds" => {
                hold = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                i += 2;
            }
            "--update-references" => {
                update_refs = true;
                i += 1;
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }

    hcs_ui::install_headless()?;
    pin_state_to_defaults();
    let mut written = 0usize;
    for spec in SPECS {
        if let Some(filter) = &only {
            if spec.name != filter {
                continue;
            }
        }
        let file = format!("{}-{}.png", spec.name, theme.as_str());
        let out = out_dir.join(&file);
        render_view(spec, theme, &out)?;
        if update_refs {
            // References are per-platform: the software renderer rasterises text
            // through the platform font stack, so a reference captured on Windows
            // is not pixel-identical on Linux and vice versa.
            let platform = if cfg!(target_os = "windows") {
                "windows"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else if cfg!(target_os = "macos") {
                "macos"
            } else {
                "other"
            };
            let refs = PathBuf::from("qa/expected/gui").join(platform);
            std::fs::create_dir_all(&refs)?;
            let refpath = refs.join(&file);
            std::fs::copy(&out, &refpath)?;
            println!("[REF] {} -> {}", out.display(), refpath.display());
        } else {
            println!("[OK] {} -> {}", spec.name, out.display());
        }
        written += 1;
    }
    println!("rendered {written} view(s), theme={}", theme.as_str());

    if hold > 0.0 {
        println!(
            "holding for {hold}s (RAM audit), rss={} MB",
            hcs_ui::process_rss_mb().unwrap_or(0)
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(hold);
        while std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
        println!("final rss={} MB", hcs_ui::process_rss_mb().unwrap_or(0));
    }
    Ok(())
}
