//! Spotlight-style Search GUI (P2, v1.2.0).
//!
//! The result pipeline is a pure function over the query so it is testable
//! without a filesystem; the .slint is view-only.

use hcs_ui::ThemePreset;
use slint::ComponentHandle;

slint::include_modules!();

hcs_ui::theme_target!(SearchWindow);

/// What a search hit can be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitKind {
    App,
    File,
    Memory,
}

impl HitKind {
    pub fn label(&self) -> &'static str {
        match self {
            HitKind::App => "app",
            HitKind::File => "file",
            HitKind::Memory => "memory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub kind: HitKind,
    pub title: String,
    pub detail: String,
}

impl Hit {
    pub fn render(&self) -> String {
        format!(
            "[{}] {}\n        {}",
            self.kind.label(),
            self.title,
            self.detail
        )
    }
}

/// The searchable catalogue. In the GUI this is seeded from installed apps and
/// the memory index; the tests use it directly.
pub fn catalog() -> Vec<Hit> {
    vec![
        Hit {
            kind: HitKind::App,
            title: "HCS Chat".into(),
            detail: "local AI chat + Image Studio".into(),
        },
        Hit {
            kind: HitKind::App,
            title: "Security Lab".into(),
            detail: "nmap, wireshark, sqlmap, john".into(),
        },
        Hit {
            kind: HitKind::App,
            title: "hcs-docs".into(),
            detail: "offline documentation viewer".into(),
        },
        Hit {
            kind: HitKind::App,
            title: "Terminal".into(),
            detail: "hcs shell".into(),
        },
        Hit {
            kind: HitKind::File,
            title: "docs/GUI_BUILD_PLAN.md".into(),
            detail: "Slint GUI plan".into(),
        },
        Hit {
            kind: HitKind::File,
            title: "render.png".into(),
            detail: "Image Studio output".into(),
        },
        Hit {
            kind: HitKind::Memory,
            title: "Q3 pentest report".into(),
            detail: "cognitive memory node".into(),
        },
        Hit {
            kind: HitKind::Memory,
            title: "tor opsec checklist".into(),
            detail: "cognitive memory node".into(),
        },
    ]
}

/// Filter the catalogue by a query (case-insensitive, matches title+detail).
pub fn search(query: &str) -> Vec<Hit> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    catalog()
        .into_iter()
        .filter(|h| h.title.to_lowercase().contains(&q) || h.detail.to_lowercase().contains(&q))
        .collect()
}

pub fn build_window() -> Result<SearchWindow, slint::PlatformError> {
    build_window_themed(ThemePreset::Obsidian)
}

/// Build the window with a theme already applied (crate-local `Palette`).
pub fn build_window_themed(theme: ThemePreset) -> Result<SearchWindow, slint::PlatformError> {
    let w = SearchWindow::new()?;
    hcs_ui::apply_hcs_theme!(w, theme);
    w.set_results("Start typing to search apps, files and cognitive memory.".into());
    w.set_source("idle".into());

    let weak = w.as_weak();
    w.on_search(move |q| {
        if let Some(w) = weak.upgrade() {
            let hits = search(&q);
            w.set_hit_count(hits.len() as i32);
            w.set_source("catalog".into());
            w.set_results(render_hits(&hits).into());
        }
    });

    let weak = w.as_weak();
    w.on_open_selected(move || {
        if let Some(w) = weak.upgrade() {
            let hits = search(&w.get_query());
            if let Some(first) = hits.first() {
                w.set_results(format!("OPEN -> {}\n\n{}", first.title, render_hits(&hits)).into());
            }
        }
    });

    let weak = w.as_weak();
    w.on_clear(move || {
        if let Some(w) = weak.upgrade() {
            w.set_query("".into());
            w.set_hit_count(0);
            w.set_source("idle".into());
            w.set_results("Start typing to search apps, files and cognitive memory.".into());
        }
    });

    Ok(w)
}

fn render_hits(hits: &[Hit]) -> String {
    if hits.is_empty() {
        return "No matches. Try apps, files or memory nodes.".to_string();
    }
    hits.iter()
        .map(|h| h.render())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn run(theme: ThemePreset) -> anyhow::Result<()> {
    let w = build_window()?;
    hcs_ui::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_nothing() {
        assert!(search("").is_empty());
        assert!(search("   ").is_empty());
    }

    #[test]
    fn matches_are_case_insensitive() {
        let hits = search("HCS");
        assert!(hits.iter().any(|h| h.title == "HCS Chat"));
        assert_eq!(search("hcs").len(), hits.len());
    }

    #[test]
    fn searches_titles_and_details() {
        assert!(search("Image Studio")
            .iter()
            .any(|h| h.title == "render.png"));
        assert!(search("sqlmap").iter().any(|h| h.title == "Security Lab"));
    }

    #[test]
    fn results_are_ranked_apps_first() {
        let hits = search("hcs");
        assert_eq!(hits[0].kind, HitKind::App, "apps rank first");
    }

    #[test]
    fn unknown_query_is_empty() {
        assert!(search("zzz-no-such-thing").is_empty());
    }
}
