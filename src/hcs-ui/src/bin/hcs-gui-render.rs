//! Headless screenshot tool for the HCS GUI foundation gate.
//!
//! Renders the foundation gallery with the Slint software renderer so
//! `scripts/verify_gui.py` can audit entropy and diff against the reference in
//! `qa/expected/gui/`. No GPU, no display, no Wayland needed.
//!
//! Usage:
//!   hcs-gui-render --out-dir qa/gui --theme obsidian
//!   hcs-gui-render --list
//!   hcs-gui-render --update-references
//!
//! App-specific views are rendered by the `hcs-gui-shots` tool, which links
//! every GUI app crate.

use hcs_ui::harness::{render_to_png, HeadlessPlatform, MinimalSoftwareWindow, RenderError};
use hcs_ui::{ThemePreset, ThemeTarget};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Every renderable view of the foundation kit, dispatched concretely
/// (no trait objects: Slint handles have associated types).
enum AnyView {
    Gallery(hcs_ui::WidgetGallery),
}

impl AnyView {
    fn name(&self) -> &'static str {
        match self {
            AnyView::Gallery(_) => "foundation-gallery",
        }
    }

    fn size(&self) -> (u32, u32) {
        match self {
            AnyView::Gallery(_) => (720, 560),
        }
    }

    fn apply_theme(&self, theme: ThemePreset) {
        let (titanium, stealth) = hcs_ui::preset_flags(theme);
        match self {
            AnyView::Gallery(g) => g.apply_theme_flags(titanium, stealth),
        }
    }

    fn render(
        &self,
        window: &Rc<MinimalSoftwareWindow>,
        w: u32,
        h: u32,
        out: &Path,
    ) -> Result<(), RenderError> {
        match self {
            AnyView::Gallery(g) => render_to_png(g, window, w, h, out),
        }
    }
}

fn views() -> Vec<AnyView> {
    vec![AnyView::Gallery(
        hcs_ui::WidgetGallery::new().expect("foundation gallery"),
    )]
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--list") {
        for v in views() {
            let (w, h) = v.size();
            println!("{}\t{}x{}", v.name(), w, h);
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
            "--gui-render" => {
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

    let window: Rc<MinimalSoftwareWindow> = HeadlessPlatform::install()?;
    let mut written = 0usize;
    for v in views() {
        if let Some(filter) = &only {
            if v.name() != filter {
                continue;
            }
        }
        v.apply_theme(theme);
        let (w, h) = v.size();
        let file = format!("{}-{}.png", v.name(), theme.as_str());
        let out = out_dir.join(&file);
        v.render(&window, w, h, &out)?;
        if update_refs {
            let refs = PathBuf::from("qa/expected/gui");
            std::fs::create_dir_all(&refs)?;
            let refpath = refs.join(&file);
            std::fs::copy(&out, &refpath)?;
            println!("[REF] {} -> {}", out.display(), refpath.display());
        } else {
            println!("[OK] {} -> {}", v.name(), out.display());
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
