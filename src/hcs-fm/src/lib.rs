//! HCS Files — directory listing, filtering and search.
//!
//! Gap closed from the v2 audit (B-07): the project had no file manager at all,
//! which is the single most conspicuous missing class of program when comparing
//! against Windows, Mint or Omarchy.
//!
//! Patterns taken:
//!   - GNOME 49 Nautilus: search with visible filter "pills" and a date
//!     narrowing control, plus `Ctrl+.` to hand the current folder to a terminal.
//!   - Windows 11 File Explorer: tabs, so several locations stay open.
//!   - macOS Finder: collapsible list, "Open With" as an explicit choice rather
//!     than a silent default.
//!
//! Everything here is pure and filesystem-agnostic behind a small trait, so the
//! listing, filtering and sorting rules are testable without touching a disk.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// What a row represents. Kinds exist because the UI must render an icon and
/// must decide what activation means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Hidden,
}

/// One row in the listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size_bytes: u64,
    /// Unix seconds; `0` when unknown (which sorts oldest).
    pub modified: i64,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Directory | EntryKind::Symlink)
    }
}

/// The active filter pills. Mirrors GNOME 49: the user must be able to *see*
/// which filters narrowed a result set, otherwise an empty pane is ambiguous
/// between "no matches" and "something failed".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filters {
    pub by_name: bool,
    pub by_content: bool,
    pub by_modified: bool,
    /// Inclusive lower bound on `modified`, in Unix seconds. `0` = no bound.
    pub modified_after: i64,
}

impl Filters {
    pub fn name_only() -> Self {
        Self {
            by_name: true,
            ..Default::default()
        }
    }

    pub fn any_active(&self) -> bool {
        self.by_name || self.by_content || self.by_modified || self.modified_after > 0
    }

    /// The first filter to switch on when the user clicks the search field, so
    /// that a fresh search always has visible, explainable scope.
    pub fn default_for(query: &str) -> Self {
        let mut f = Self::name_only();
        if query.is_empty() {
            return f;
        }
        // A query that looks like a date (YYYY-MM-DD) narrows by modification,
        // which is the single most useful date filter and needs no calendar.
        if looks_like_date(query) {
            f.by_modified = true;
            f.modified_after = 0;
            f.by_name = false;
        }
        f
    }
}

fn looks_like_date(q: &str) -> bool {
    let b = q.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

/// Sort column, matching the view's header order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortKey {
    Name,
    Size,
    Modified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SortSpec {
    pub key: SortKey,
    pub descending: bool,
}

impl Default for SortSpec {
    fn default() -> Self {
        // Directories first, then case-insensitive by name — the ordering every
        // file manager agrees on, because it is the only one that keeps a
        // folder's contents together.
        Self {
            key: SortKey::Name,
            descending: false,
        }
    }
}

/// Ordering function. Separated from the view so it can be asserted directly.
pub fn compare_entries(a: &Entry, b: &Entry, spec: &SortSpec) -> Ordering {
    // Directories always lead, in *both* directions. Reversing a directory into
    // the middle of a listing is disorienting, and every file manager agrees on
    // this. So the dir/file comparison is inverted relative to the value
    // comparison and is deliberately not touched by `descending`.
    let dir_order = b.is_dir().cmp(&a.is_dir());
    if dir_order != std::cmp::Ordering::Equal {
        return dir_order;
    }

    let value_order = match spec.key {
        SortKey::Name => a
            .name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Size => a
            .size_bytes
            .cmp(&b.size_bytes)
            .then_with(|| a.name.cmp(&b.name)),
        SortKey::Modified => a
            .modified
            .cmp(&b.modified)
            .then_with(|| a.name.cmp(&b.name)),
    };
    if spec.descending {
        value_order.reverse()
    } else {
        value_order
    }
}

/// Apply the filters and sort. One function, so the view cannot accidentally
/// filter without sorting or sort without filtering.
pub fn apply(entries: &[Entry], query: &str, filters: &Filters, spec: &SortSpec) -> Vec<Entry> {
    let q = query.trim().to_lowercase();
    let mut out: Vec<Entry> = entries
        .iter()
        .filter(|e| {
            if !filters.by_modified && filters.modified_after == 0 {
                // no date narrowing requested
            } else if e.modified < filters.modified_after {
                return false;
            }
            if q.is_empty() {
                return true;
            }
            let name_hit = filters.by_name && e.name.to_lowercase().contains(&q);
            // Content search is delegated to the caller via `content_index`;
            // this crate only owns name and date filtering.
            let _ = filters.by_content;
            name_hit
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| compare_entries(a, b, spec));
    out
}

/// Content search needs the bytes, which the pure layer does not have. This
/// keeps the rule in one place instead of being reimplemented per view.
pub fn matches_content(
    entry: &Entry,
    needle: &str,
    read: &dyn Fn(&Path) -> Option<String>,
) -> bool {
    if needle.trim().is_empty() {
        return true;
    }
    if entry.is_dir() {
        return false;
    }
    let n = needle.to_lowercase();
    read(&entry.path)
        .map(|c| c.to_lowercase().contains(&n))
        .unwrap_or(false)
}

/// Minimal directory reading behind a trait, so tests supply fixtures and the
/// binary uses the real filesystem.
pub trait DirectorySource {
    fn list(&self, path: &Path) -> anyhow::Result<Vec<Entry>>;
}

/// The real filesystem.
pub struct FsSource;

impl DirectorySource for FsSource {
    fn list(&self, path: &Path) -> anyhow::Result<Vec<Entry>> {
        let mut out = Vec::new();
        if !path.exists() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata().ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_symlink = entry.file_type().map(|t| t.is_symlink()).unwrap_or(false);
            let hidden = name.starts_with('.');
            let kind = if is_symlink {
                EntryKind::Symlink
            } else if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) {
                EntryKind::Directory
            } else {
                EntryKind::File
            };
            let modified = meta
                .as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            out.push(Entry {
                name,
                path: entry.path(),
                kind,
                size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                modified,
            });
            let _ = hidden;
        }
        Ok(out)
    }
}

/// Where an activation should send a file. Extracted so the choice is explicit
/// (macOS "Open With") rather than an invisible default association.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWithChoice {
    pub program: String,
    pub args: Vec<String>,
}

/// Map a file extension to the program that should handle it. Deliberately
/// short: an unknown type falls through to the viewer, never to a guess.
pub fn default_program_for(entry: &Entry) -> Option<OpenWithChoice> {
    let ext = entry.path.extension()?.to_string_lossy().to_lowercase();
    let program = match ext.as_str() {
        "md" | "markdown" => "hcs-notes",
        "txt" | "log" | "conf" | "toml" | "json" | "yaml" | "yml" | "kdl" => "hcs-notes",
        "png" | "jpg" | "jpeg" | "webp" => "hcs-shot",
        "sh" | "bash" => "hcs-term",
        _ => return None,
    };
    Some(OpenWithChoice {
        program: program.to_string(),
        args: vec![entry.path.to_string_lossy().to_string()],
    })
}

/// The hand-off command for `Ctrl+.` (GNOME 49 Nautilus behaviour).
pub fn terminal_command_for(path: &Path) -> String {
    format!("cd {}", shell_quote(&path.to_string_lossy()))
}

/// Single-quote a path for `sh -c`. A path with a space or a quote in it must
/// not be able to break out of the command.
pub fn shell_quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=+:@~".contains(c))
    {
        return s.to_string();
    }
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Human-readable size, matching what the column header promises.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// Format a Unix timestamp for the "Date" column. Falls back to the raw value
/// rather than an empty cell, because a blank date reads as missing data.
pub fn human_date(unix: i64) -> String {
    if unix <= 0 {
        return "-".to_string();
    }
    chrono::DateTime::from_timestamp(unix, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| unix.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, dir: bool, size: u64, modified: i64) -> Entry {
        Entry {
            name: name.to_string(),
            path: PathBuf::from("/tmp").join(name),
            kind: if dir {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            size_bytes: size,
            modified,
        }
    }

    #[test]
    fn directories_sort_before_files() {
        let entries = vec![entry("zebra.txt", false, 10, 5), entry("alpha", true, 0, 5)];
        let out = apply(&entries, "", &Filters::default(), &SortSpec::default());
        assert_eq!(out[0].name, "alpha");
        assert_eq!(out[1].name, "zebra.txt");
    }

    #[test]
    fn directories_stay_first_even_when_descending() {
        let entries = vec![entry("zebra.txt", false, 10, 5), entry("alpha", true, 0, 5)];
        let spec = SortSpec {
            key: SortKey::Name,
            descending: true,
        };
        let out = apply(&entries, "", &Filters::default(), &spec);
        assert!(out[0].is_dir(), "directory must still lead when reversed");
    }

    #[test]
    fn name_sort_is_case_insensitive_with_stable_tiebreak() {
        let entries = vec![
            entry("Beta", false, 1, 1),
            entry("alpha", false, 1, 1),
            entry("Beta.txt", false, 1, 1),
        ];
        let out = apply(&entries, "", &Filters::default(), &SortSpec::default());
        let names: Vec<_> = out.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "Beta", "Beta.txt"]);
    }

    #[test]
    fn sort_by_size_orders_numerically() {
        let entries = vec![
            entry("small", false, 10, 1),
            entry("big", false, 100_000, 1),
            entry("mid", false, 900, 1),
        ];
        let spec = SortSpec {
            key: SortKey::Size,
            descending: false,
        };
        let out = apply(&entries, "", &Filters::default(), &spec);
        let names: Vec<_> = out.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["small", "mid", "big"]);
    }

    #[test]
    fn name_filter_matches_substring_case_insensitively() {
        let entries = vec![
            entry("Report.md", false, 1, 1),
            entry("binary.bin", false, 1, 1),
        ];
        let out = apply(
            &entries,
            "report",
            &Filters::name_only(),
            &SortSpec::default(),
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Report.md");
    }

    #[test]
    fn date_filter_excludes_older_entries() {
        let entries = vec![
            entry("old.txt", false, 1, 100),
            entry("new.txt", false, 1, 5_000),
        ];
        let filters = Filters {
            by_modified: true,
            modified_after: 1_000,
            ..Default::default()
        };
        let out = apply(&entries, "", &filters, &SortSpec::default());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "new.txt");
    }

    #[test]
    fn empty_query_with_filters_keeps_entries() {
        let entries = vec![entry("a", false, 1, 5_000)];
        let out = apply(&entries, "   ", &Filters::name_only(), &SortSpec::default());
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn default_filters_switch_to_date_for_a_date_query() {
        let f = Filters::default_for("2026-09-30");
        assert!(f.by_modified);
        assert!(!f.by_name);
    }

    #[test]
    fn default_filters_stay_name_only_for_normal_query() {
        let f = Filters::default_for("report");
        assert!(f.by_name);
        assert!(!f.by_modified);
    }

    #[test]
    fn filters_report_whether_anything_is_active() {
        assert!(!Filters::default().any_active());
        assert!(Filters::name_only().any_active());
    }

    #[test]
    fn content_search_delegates_to_reader() {
        let e = entry("notes.md", false, 1, 1);
        let read = |p: &Path| {
            if p.ends_with("notes.md") {
                Some("# Secret heading about tor".to_string())
            } else {
                None
            }
        };
        assert!(matches_content(&e, "tor", &read));
        assert!(!matches_content(&e, "absent", &read));
    }

    #[test]
    fn content_search_skips_directories() {
        let d = entry("folder", true, 0, 1);
        let read = |_: &Path| Some("anything".to_string());
        assert!(!matches_content(&d, "anything", &read));
    }

    #[test]
    fn known_extension_maps_to_a_program() {
        let e = entry("notes.md", false, 1, 1);
        let choice = default_program_for(&e).expect("markdown must map");
        assert_eq!(choice.program, "hcs-notes");
    }

    #[test]
    fn unknown_extension_maps_to_nothing_rather_than_guessing() {
        let e = entry("thing.qqq", false, 1, 1);
        assert!(default_program_for(&e).is_none());
    }

    #[test]
    fn extensionless_file_maps_to_nothing() {
        let e = entry("LICENSE", false, 1, 1);
        assert!(default_program_for(&e).is_none());
    }

    #[test]
    fn shell_quote_leaves_simple_paths_alone() {
        assert_eq!(shell_quote("/home/hcs/Documents"), "/home/hcs/Documents");
    }

    #[test]
    fn shell_quote_escapes_spaces() {
        assert_eq!(shell_quote("/home/hcs/My Docs"), "'/home/hcs/My Docs'");
    }

    #[test]
    fn shell_quote_neutralises_embedded_single_quote() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn shell_quote_rejects_command_substitution() {
        let q = shell_quote("$(rm -rf /)");
        assert!(q.starts_with('\''));
        assert!(q.ends_with('\''));
    }

    #[test]
    fn terminal_command_quotes_the_path() {
        let cmd = terminal_command_for(Path::new("/home/hcs/My Docs"));
        assert_eq!(cmd, "cd '/home/hcs/My Docs'");
    }

    #[test]
    fn human_size_scales_units() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn human_date_never_renders_blank() {
        assert_eq!(human_date(0), "-");
        assert_eq!(human_date(-5), "-");
        assert!(human_date(1_700_000_000).contains("2023"));
    }

    #[test]
    fn filesystem_source_lists_a_temp_dir() {
        let dir = std::env::temp_dir().join("hcs-fm-test");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("file.txt"), b"hello").unwrap();

        let entries = FsSource.list(&dir).unwrap();
        let mut names: Vec<_> = entries.iter().map(|e| e.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["file.txt", "sub"]);

        let file = entries.iter().find(|e| e.name == "file.txt").unwrap();
        assert_eq!(file.size_bytes, 5);
        assert!(!file.is_dir());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn filesystem_source_returns_empty_for_missing_dir() {
        let out = FsSource.list(Path::new("/definitely/not/here")).unwrap();
        assert!(out.is_empty());
    }
}
