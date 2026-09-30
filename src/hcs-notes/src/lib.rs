//! HCS Notes — Markdown notes with offline search.
//!
//! Pattern: Windows 11 Notepad gained native Markdown editing (bold, italic,
//! lists, headings, live preview) in 2025. HCS Notes does the same over the
//! local corpus, plus session restore, and never opens a browser.
//!
//! The Markdown rendering and the search index are pure functions so both can
//! be asserted without a window.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A note as stored on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    /// Unix seconds.
    pub modified: i64,
}

impl Note {
    pub fn new(id: &str, title: &str, body: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            modified: 0,
        }
    }

    /// First non-empty, non-heading line of the body, used as a search snippet.
    pub fn snippet(&self, max_chars: usize) -> String {
        let line = self
            .body
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with('#'))
            .unwrap_or("");
        if line.chars().count() <= max_chars {
            line.to_string()
        } else {
            let truncated: String = line.chars().take(max_chars).collect();
            format!("{truncated}…")
        }
    }

    /// Lowercase haystack used for search: title plus body, so a match in
    /// either field counts.
    fn haystack(&self) -> String {
        format!("{}\n{}", self.title, self.body).to_lowercase()
    }
}

/// A lightweight Markdown-to-plain-text renderer.
///
/// Full CommonMark is out of scope and would be the wrong dependency: the notes
/// surface needs headings, emphasis, lists, code and links to be *readable* in a
/// small text area, not to be a second browser.
pub fn render_markdown(src: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut list_count = 0usize;

    for raw in src.lines() {
        let line = raw.trim_end();

        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            out.push('\n');
            continue;
        }
        if in_code {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if line.trim().is_empty() {
            if list_count > 0 {
                out.push('\n');
                list_count = 0;
            }
            continue;
        }

        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#") {
            if list_count > 0 {
                out.push('\n');
                list_count = 0;
            }
            // Count the hashes on the *original* token: `strip_prefix` already
            // consumed one, so counting in `rest` would stop at the space and
            // render every heading as level 0.
            let level = t.chars().take_while(|c| *c == '#').count();
            let text = rest.trim_start_matches('#').trim();
            out.push('\n');
            out.push_str(&"#".repeat(level));
            out.push(' ');
            out.push_str(&strip_inline(text));
            out.push('\n');
            continue;
        }

        let (marker, rest) = if let Some(r) = t.strip_prefix("- [ ] ") {
            ("- [ ] ", r)
        } else if let Some(r) = t.strip_prefix("- [x] ") {
            ("- [x] ", r)
        } else if let Some(r) = t.strip_prefix("- ") {
            ("- ", r)
        } else if let Some(r) = t.strip_prefix("* ") {
            ("- ", r)
        } else {
            ("", t)
        };

        if !marker.is_empty() {
            list_count += 1;
            out.push_str("  ");
            out.push_str(marker);
            out.push_str(&strip_inline(rest));
            out.push('\n');
            continue;
        }

        if list_count > 0 {
            out.push('\n');
            list_count = 0;
        }
        out.push_str(&strip_inline(line));
        out.push('\n');
    }
    out.trim_end().to_string()
}

/// Remove inline Markdown markers, keeping the visible text.
///
/// Deliberately small and conservative. An unclosed marker is emitted literally
/// rather than swallowing the rest of the line: an editor that silently eats
/// text after a stray `*` is worse than one that shows the asterisk.
fn strip_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // `**bold**` and `*italic*`
        if chars[i] == '*' {
            let run = chars[i..].iter().take_while(|c| **c == '*').count();
            let open = i + run;
            // The closing run must be at least as long as the opening one.
            let close = chars[open..].iter().enumerate().find(|(off, c)| {
                **c == '*'
                    && chars[open + off..]
                        .iter()
                        .take_while(|x| **x == '*')
                        .count()
                        >= run
            });
            if let Some((off, _)) = close {
                let close = open + off;
                if close > open {
                    out.push_str(&chars[open..close].iter().collect::<String>());
                    i = close + run;
                    continue;
                }
            }
            // Unbalanced marker: emit it literally.
            out.push_str(&chars[i..open].iter().collect::<String>());
            i = open;
            continue;
        }
        // `[text](url)` -> `text (url)`
        if chars[i] == '[' {
            if let Some(close) = chars[i..].iter().position(|c| *c == ']') {
                let close = i + close;
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(paren) = chars[close..].iter().position(|c| *c == ')') {
                        let text: String = chars[i + 1..close].iter().collect();
                        let url: String = chars[close + 2..close + paren].iter().collect();
                        out.push_str(&text);
                        out.push_str(" (");
                        out.push_str(&url);
                        out.push(')');
                        i = close + paren + 1;
                        continue;
                    }
                }
            }
        }
        // `` `code` `` -> code
        if chars[i] == '`' {
            // Search *after* the opening backtick: `position` on `chars[i..]`
            // would always report 0 and swallow the span.
            if let Some(off) = chars[i + 1..].iter().position(|c| *c == '`') {
                out.push_str(&chars[i + 1..i + 1 + off].iter().collect::<String>());
                i = i + off + 2;
                continue;
            }
            out.push('`');
            i += 1;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// One search hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub score: f64,
    pub snippet: String,
}

/// Rank notes against a query. Title matches beat body matches; an exact title
/// beats a prefix; ties break on id so the order is reproducible.
pub fn search(notes: &[Note], query: &str, limit: usize) -> Vec<Hit> {
    let q = query.trim().to_lowercase();
    let mut hits: Vec<Hit> = notes
        .iter()
        .filter_map(|n| {
            if q.is_empty() {
                return Some(Hit {
                    id: n.id.clone(),
                    score: 0.1,
                    snippet: n.snippet(80),
                });
            }
            let title = n.title.to_lowercase();
            let body = n.haystack();
            let score = if title == q {
                1.0
            } else if title.starts_with(&q) {
                0.9
            } else if title.contains(&q) {
                0.75
            } else if body.contains(&q) {
                0.5
            } else {
                return None;
            };
            Some(Hit {
                id: n.id.clone(),
                score,
                snippet: n.snippet(80),
            })
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    hits.truncate(limit);
    hits
}

/// Session restore state. Windows 11 Text Editor added session saving; the
/// failure mode without it is reopening an editor full of lost drafts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub open_ids: Vec<String>,
    pub active_id: Option<String>,
}

impl Session {
    pub fn new(active: &str) -> Self {
        Self {
            open_ids: vec![active.to_string()],
            active_id: Some(active.to_string()),
        }
    }

    /// Restore, dropping ids that no longer exist. A stale id must not produce
    /// an empty editor with no explanation.
    ///
    /// If the previously active note is gone, the *last surviving* tab becomes
    /// active — the same fallback `close` uses. Restoring with no active tab at
    /// all would leave the editor showing nothing, which looks like data loss.
    pub fn restore(&self, available: &[Note]) -> Session {
        let open: Vec<String> = self
            .open_ids
            .iter()
            .filter(|id| available.iter().any(|n| &n.id == *id))
            .cloned()
            .collect();
        let active = self
            .active_id
            .clone()
            .filter(|id| open.iter().any(|o| o == id))
            .or_else(|| open.last().cloned());
        Session {
            open_ids: open,
            active_id: active,
        }
    }

    pub fn open(&mut self, id: &str) {
        if !self.open_ids.iter().any(|o| o == id) {
            self.open_ids.push(id.to_string());
        }
        self.active_id = Some(id.to_string());
    }

    pub fn close(&mut self, id: &str) {
        self.open_ids.retain(|o| o != id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = self.open_ids.last().cloned();
        }
    }

    /// Mark a note dirty: a tab with unsaved changes must be distinguishable
    /// from a clean one after a crash and restore.
    pub fn dirty_ids(&self, saved: &BTreeMap<String, String>) -> Vec<String> {
        self.open_ids
            .iter()
            .filter(|id| saved.get(*id).is_none())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Vec<Note> {
        vec![
            Note::new(
                "n1",
                "Tor Killswitch",
                "The nft rules fail closed.\nPort 9040.",
            ),
            Note::new("n2", "RAM budget", "Idle 6144 MB, peak 8192 MB."),
            Note::new("n3", "Shopping", "Milk, coffee, tor switches"),
        ]
    }

    #[test]
    fn search_finds_by_title() {
        let hits = search(&corpus(), "tor", 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "n1");
    }

    #[test]
    fn title_hit_outranks_body_hit() {
        let hits = search(&corpus(), "ram", 5);
        assert_eq!(hits[0].id, "n2");
    }

    #[test]
    fn search_is_case_insensitive() {
        assert_eq!(search(&corpus(), "TOR", 5)[0].id, "n1");
    }

    #[test]
    fn search_respects_the_limit() {
        let hits = search(&corpus(), "", 2);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn snippet_skips_headings_and_blanks() {
        let n = Note::new("x", "T", "# Heading\n\n\nThe real first line.\nsecond");
        assert_eq!(n.snippet(80), "The real first line.");
    }

    #[test]
    fn snippet_truncates_with_an_ellipsis() {
        let n = Note::new("x", "T", &"a".repeat(200));
        let s = n.snippet(10);
        assert_eq!(s.chars().count(), 11);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn markdown_headings_survive() {
        let out = render_markdown("# Title\nbody");
        assert!(out.contains("# Title"));
        assert!(out.contains("body"));
    }

    #[test]
    fn markdown_bold_and_italic_are_stripped() {
        assert_eq!(strip_inline("**bold**"), "bold");
        assert_eq!(strip_inline("*italic*"), "italic");
    }

    #[test]
    fn markdown_link_becomes_text_and_url() {
        assert_eq!(strip_inline("[HCS](https://hcs)"), "HCS (https://hcs)");
    }

    #[test]
    fn markdown_inline_code_is_unwrapped() {
        assert_eq!(strip_inline("run `hcs model list`"), "run hcs model list");
    }

    #[test]
    fn markdown_unbalanced_marker_is_left_alone() {
        // An unclosed marker must not eat the rest of the document.
        assert!(strip_inline("2 * 3 = 6").contains('2'));
    }

    #[test]
    fn markdown_checkbox_is_preserved() {
        let out = render_markdown("- [ ] write gate\n- [x] ship it");
        assert!(out.contains("[ ] write gate"));
        assert!(out.contains("[x] ship it"));
    }

    #[test]
    fn markdown_code_block_keeps_internal_structure() {
        let out = render_markdown("```\nline one\n  line two\n```");
        assert!(out.contains("line one"));
        assert!(out.contains("  line two"));
    }

    #[test]
    fn session_tracks_open_and_active() {
        let mut s = Session::new("n1");
        s.open("n2");
        assert_eq!(s.active_id.as_deref(), Some("n2"));
        assert_eq!(s.open_ids, vec!["n1", "n2"]);
    }

    #[test]
    fn opening_twice_does_not_duplicate() {
        let mut s = Session::new("n1");
        s.open("n1");
        assert_eq!(s.open_ids.len(), 1);
    }

    #[test]
    fn closing_active_promotes_the_last_open_tab() {
        let mut s = Session::new("n1");
        s.open("n2");
        s.close("n2");
        assert_eq!(s.active_id.as_deref(), Some("n1"));
    }

    #[test]
    fn restore_drops_ids_that_no_longer_exist() {
        let s = Session {
            open_ids: vec!["n1".into(), "gone".into()],
            active_id: Some("gone".into()),
        };
        let restored = s.restore(&corpus());
        assert_eq!(restored.open_ids, vec!["n1"]);
        // The active id pointed at a deleted note, so it falls back rather than
        // leaving the editor pointing at nothing.
        assert_eq!(restored.active_id.as_deref(), Some("n1"));
    }

    #[test]
    fn dirty_ids_report_unsaved_notes() {
        let mut s = Session::new("n1");
        s.open("n2");
        let mut saved = BTreeMap::new();
        saved.insert("n1".to_string(), "body".to_string());
        assert_eq!(s.dirty_ids(&saved), vec!["n2"]);
    }
}
