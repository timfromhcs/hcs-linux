//! Native offline documentation viewer (P3, v1.3.0).
//!
//! Markdown is parsed in Rust with `pulldown-cmark` and rendered as styled
//! plain text. No webview, no browser embed, no network — the manuals ship in
//! `/usr/share/hcs/docs/manuals` and the viewer must work on a dead network.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ThemePreset;

slint::include_modules!();

crate::theme_target!(DocsWindow);

pub const DOCS_DIR: &str = "/usr/share/hcs/docs/manuals";
/// Repo-local manuals, used on a development machine.
pub const DOCS_DIR_LOCAL: &str = "config/includes.chroot/usr/share/hcs/docs/manuals";

/// The six manuals the plan requires (Pillar 7).
pub const MANUALS: &[(&str, &str, &str)] = &[
    (
        "01_getting_started",
        "01 Getting Started",
        "Installation, desktop tour, basics",
    ),
    (
        "02_ai_brain_guide",
        "02 AI Brain Guide",
        "Model tiers, MCP tools, memory",
    ),
    (
        "03_cpu_image_studio",
        "03 CPU Image Studio",
        "txt2img / img2img on pure CPU",
    ),
    (
        "04_security_pentest",
        "04 Security Pentest",
        "Debian-native tool arsenal",
    ),
    (
        "05_tor_anonymity",
        "05 Tor Anonymity",
        "Fail-closed nftables + OpSec",
    ),
    (
        "06_developer_manual",
        "06 Developer Manual",
        "Runtimes, MCP authoring",
    ),
];

/// Resolve the manuals directory: on-device first, then the repo checkout.
pub fn manuals_dir() -> PathBuf {
    let dev = Path::new(DOCS_DIR);
    if dev.is_dir() {
        return dev.to_path_buf();
    }
    PathBuf::from(DOCS_DIR_LOCAL)
}

/// Convert markdown to plain, readable text for the Slint Text element.
pub fn markdown_to_text(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, opts);
    let mut out = String::new();
    let mut in_code = false;
    let mut list_prefix = false;

    for ev in parser {
        match ev {
            Event::Start(Tag::Heading { .. }) => out.push_str("\n### "),
            Event::End(TagEnd::Heading(_)) => out.push('\n'),
            Event::Start(Tag::Paragraph) => {
                if out.is_empty() || out.ends_with('\n') {
                    // no leading blank line for the first paragraph
                } else {
                    out.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph) => out.push('\n'),
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                out.push_str("\n    ");
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code = false;
                out.push('\n');
            }
            Event::Start(Tag::List(_)) => {
                list_prefix = true;
            }
            Event::End(TagEnd::List(_)) => {
                list_prefix = false;
                out.push('\n');
            }
            Event::Start(Tag::Item) => {
                out.push_str("  - ");
                list_prefix = true;
            }
            Event::End(TagEnd::Item) => {
                out.push('\n');
            }
            Event::Text(t) | Event::Code(t) => {
                if in_code {
                    out.push_str(&t.replace('\n', "\n    "));
                } else {
                    out.push_str(&t);
                }
            }
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("\n--------\n"),
            _ => {}
        }
    }
    let _ = list_prefix;
    out.trim().to_string()
}

/// Read and render one manual. Falls back to a helpful message when the file
/// is missing, so the window never shows an empty pane.
pub fn render_manual(stem: &str) -> (String, String) {
    let path = manuals_dir().join(format!("{stem}.md"));
    match std::fs::read_to_string(&path) {
        Ok(md) => {
            let title = MANUALS
                .iter()
                .find(|(s, _, _)| *s == stem)
                .map(|(_, t, _)| (*t).to_string())
                .unwrap_or_else(|| stem.to_string());
            (title, markdown_to_text(&md))
        }
        Err(e) => (
            stem.to_string(),
            format!("Manual not found: {}\n\nLooked in: {}", path.display(), e),
        ),
    }
}

pub fn build_window() -> Result<DocsWindow, slint::PlatformError> {
    build_window_themed(ThemePreset::Obsidian)
}

/// Build the window with a theme already applied.
pub fn build_window_themed(theme: ThemePreset) -> Result<DocsWindow, slint::PlatformError> {
    let w = build_window_from(manuals_dir())?;
    crate::apply_hcs_theme!(w, theme);
    Ok(w)
}

pub fn build_window_from(dir: PathBuf) -> Result<DocsWindow, slint::PlatformError> {
    let w = DocsWindow::new()?;
    let list: String = MANUALS
        .iter()
        .map(|(_, title, desc)| format!("{title}\n    {desc}"))
        .collect::<Vec<_>>()
        .join("\n");
    w.set_manual_list(list.into());
    w.set_manual_count(MANUALS.len() as i32);
    w.set_rendered(
        "Select a manual on the left.\n\nAll manuals are stored offline at /usr/share/hcs/docs/manuals \
         and rendered natively (no browser, no network)."
            .into(),
    );

    let weak = w.as_weak();
    w.on_open(move |file| {
        if let Some(w) = weak.upgrade() {
            let (title, body) = render_manual(&file);
            w.set_current_title(title.into());
            w.set_current_file(file);
            w.set_rendered(body.into());
        }
    });

    let weak = w.as_weak();
    w.on_explain(move || {
        if let Some(w) = weak.upgrade() {
            let current = w.get_current_file().to_string();
            w.set_rendered(
                format!(
                    "Explain with HCS Brain (offline, Qwen3-0.6B resident)\n\n\
                     Kontext: manual '{}'\n\nDie Erklaerung laeuft vollstaendig lokal - kein Netzwerk noetig. \
                     Starten Sie hcs-chat und fragen Sie nach dem gewaehlten Abschnitt.",
                    current
                )
                .into(),
            );
        }
    });

    let _ = dir;
    Ok(w)
}

pub fn run(theme: ThemePreset) -> anyhow::Result<()> {
    let w = build_window()?;
    crate::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}

/// Keep the Rc import used by the callback wiring honest.
pub type SharedDocs = Rc<()>;
