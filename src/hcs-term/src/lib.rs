//! `hcs-term` — the terminal emulator.
//!
//! Gap closed from the v2 audit (B-08): `cheatsheet.json` promised "Terminal" on
//! HCS+T, but no binary existed to open a window. The shell still binds `Mod+T`.
//!
//! Profile search follows the GNOME 49 Ptyxis pattern (`Alt+,` searches
//! containers and profiles). For HCS the "containers" are the AI profiles and dev
//! environments, so the picker lists those rather than Docker containers.
//!
//! The emulator core is deliberately a pure state machine over a cell grid:
//! escape sequences mutate the grid, the grid is rendered by Slint. That split
//! is what lets the grid logic be unit-tested with no display attached.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// CSI parameters for erase operations, shared by EL and ED so the two cannot
/// drift apart. Names come from ECMA-48's erase functions.
const CSI_ERASE_TO_END: u8 = 0;
const CSI_ERASE_ALL: u8 = 2;
const CSI_ERASE_TO_START: u8 = 1;

/// Rows the window renders. Exported so the view and the grid cannot disagree
/// about how many lines exist.
pub const ROWS: u32 = 30;

/// Columns the window renders at the default 900px width.
pub const COLS: usize = 96;

/// A single character cell: the terminal is a grid of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// A blank cell holds a space, not NUL. `char::default()` is `\0`, which
    /// `trim_end` does not remove, so every row would carry a tail of invisible
    /// NULs into the view and into the regression renders.
    pub ch: char,
    /// Foreground colour as #rrggbb; `None` means "terminal default".
    pub fg: Option<u32>,
    pub bg: Option<u32>,
    pub bold: bool,
    pub underline: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank()
    }
}

impl Cell {
    pub fn blank() -> Self {
        Self {
            ch: ' ',
            fg: None,
            bg: None,
            bold: false,
            underline: false,
        }
    }
}

/// Terminal colours in the 16 ANSI slots plus explicit bright variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub foreground: u32,
    pub background: u32,
    pub cursor: u32,
    pub ansi: [u32; 16],
}

impl Default for Palette {
    /// Obsidian theme colours, matching the hcs-ui palette so the terminal looks
    /// like part of the desktop rather than a guest in it.
    fn default() -> Self {
        Self {
            foreground: 0xE6_ED_F3,
            background: 0x0D_11_17,
            cursor: 0x38_BD_F8,
            // ANSI slots 0-15. Written as plain hex because `0x56_D3_64` reads
            // as a mistyped `i64` suffix to the compiler and to clippy alike.
            ansi: [
                0x1C2430, 0xF85149, 0x3FB950, 0xF59E0B, 0x38BDF8, 0xA78BFA, 0x34D399, 0xE6EDF3,
                0x30363D, 0xFF7B72, 0x56D364, 0xFBBF24, 0x7DD3FC, 0xC4B5FD, 0x6EE7B7, 0xF0F6FC,
            ],
        }
    }
}

/// A terminal grid with a cursor and scrollback accounting.
#[derive(Debug, Clone)]
pub struct Grid {
    cols: usize,
    rows: usize,
    cells: Vec<Cell>,
    cursor_row: usize,
    cursor_col: usize,
    palette: Palette,
    scrollback_limit: usize,
    /// Current SGR attributes applied to newly written cells.
    current_fg: Option<u32>,
    current_bold: bool,
    current_underline: bool,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(2);
        let rows = rows.max(1);
        Self {
            cols,
            rows,
            cells: vec![Cell::blank(); cols * rows],
            cursor_row: 0,
            cursor_col: 0,
            palette: Palette::default(),
            // 200 rows keeps RSS inside the 220 MB budget from plan §6.
            scrollback_limit: 200,
            current_fg: None,
            current_bold: false,
            current_underline: false,
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn cell(&self, row: usize, col: usize) -> Cell {
        if row >= self.rows || col >= self.cols {
            return Cell::blank();
        }
        self.cells[row * self.cols + col]
    }

    pub fn row_text(&self, row: usize) -> String {
        (0..self.cols)
            .map(|c| self.cell(row, c).ch)
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    pub fn text(&self) -> String {
        (0..self.rows)
            .map(|r| self.row_text(r))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn scroll_up_one(&mut self) {
        self.cells.drain(0..self.cols);
        let blank = vec![Cell::blank(); self.cols];
        self.cells.extend(blank);
        self.cursor_row = self.cursor_row.saturating_sub(1);
    }

    /// Write a character at the cursor and advance. This is the single place
    /// where cursor movement happens, so wrapping behaviour is testable alone.
    pub fn put(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
        }
        while self.cursor_row >= self.rows {
            self.scroll_up_one();
            self.cursor_row = self.rows - 1;
        }
        let idx = self.cursor_row * self.cols + self.cursor_col;
        self.cells[idx] = Cell {
            ch,
            fg: self.current_fg,
            bg: None,
            bold: self.current_bold,
            underline: self.current_underline,
        };
        self.cursor_col += 1;
    }

    pub fn write_str(&mut self, s: &str) {
        for ch in s.chars() {
            if ch == '\n' {
                self.newline();
            } else if ch == '\r' {
                self.cursor_col = 0;
            } else if ch == '\t' {
                let next = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next.min(self.cols);
            } else if !ch.is_control() {
                self.put(ch);
            }
        }
    }

    pub fn newline(&mut self) {
        self.cursor_row += 1;
        self.cursor_col = 0;
        while self.cursor_row >= self.rows {
            self.scroll_up_one();
            self.cursor_row = self.rows - 1;
        }
    }

    pub fn set_fg(&mut self, ansi_index: Option<u8>, rgb: Option<u32>) {
        self.current_fg = match (rgb, ansi_index) {
            (Some(c), _) => Some(c),
            (None, Some(i)) => self.palette.ansi.get(i as usize).copied(),
            _ => None,
        };
    }

    pub fn set_bold(&mut self, on: bool) {
        self.current_bold = on;
    }

    pub fn set_underline(&mut self, on: bool) {
        self.current_underline = on;
    }

    pub fn clear(&mut self) {
        self.cells.iter_mut().for_each(|c| *c = Cell::blank());
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Handle the subset of escape sequences that matters for readable output:
    /// colour selection, bold/underline reset, erase-line and erase-display.
    /// Anything unrecognised is ignored rather than corrupting the grid.
    pub fn feed(&mut self, bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        let mut chars = s.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '\u{1b}' => {
                    if chars.peek() == Some(&'[') {
                        chars.next();
                        let mut seq = String::new();
                        while let Some(&c) = chars.peek() {
                            if c.is_ascii_alphabetic() {
                                break;
                            }
                            seq.push(c);
                            chars.next();
                        }
                        let final_byte = chars.next().unwrap_or('m');
                        self.apply_csi(&seq, final_byte);
                    }
                }
                c => self.put(c),
            }
        }
    }

    fn apply_csi(&mut self, params: &str, final_byte: char) {
        let nums: Vec<u8> = params
            .split(';')
            .map(|p| p.trim().parse::<u8>().unwrap_or(0))
            .collect();
        match final_byte {
            'm' => {
                if nums.is_empty() {
                    self.set_fg(None, None);
                    self.set_bold(false);
                    self.set_underline(false);
                    return;
                }
                for n in nums {
                    match n {
                        0 => {
                            self.set_fg(None, None);
                            self.set_bold(false);
                            self.set_underline(false);
                        }
                        1 => self.set_bold(true),
                        4 => self.set_underline(true),
                        22 => self.set_bold(false),
                        24 => self.set_underline(false),
                        30..=37 => self.set_fg(Some(n - 30), None),
                        90..=97 => self.set_fg(Some(n - 90 + 8), None),
                        _ => {}
                    }
                }
            }
            'K' => match nums.first().copied().unwrap_or(0) {
                // Erase to end of line.
                0 => {
                    let start = self.cursor_row * self.cols + self.cursor_col;
                    for c in self.cells[start..].iter_mut() {
                        *c = Cell::blank();
                    }
                }
                // Erase the whole line.
                CSI_ERASE_ALL => {
                    let start = self.cursor_row * self.cols;
                    for c in self.cells[start..(start + self.cols)].iter_mut() {
                        *c = Cell::blank();
                    }
                }
                _ => {}
            },
            // Erase display (ED). Only "erase everything" is implemented; the
            // partial variants are listed explicitly and ignored rather than
            // guessed at, because a wrong guess would visibly corrupt the
            // screen while looking like a successful clear.
            'J' => match nums.first().copied().unwrap_or(0) {
                CSI_ERASE_ALL => self.clear(),
                CSI_ERASE_TO_END | CSI_ERASE_TO_START => {}
                _ => {}
            },
            _ => {}
        }
    }

    pub fn scrollback_limit(&self) -> usize {
        self.scrollback_limit
    }
}

/// A launchable profile. The picker lists these; GNOME 49's Ptyxis does the
/// same for containers, and for HCS the useful "containers" are AI profiles and
/// dev environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub shell: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: BTreeMap<String, String>,
}

impl Profile {
    pub fn new(id: &str, name: &str, shell: &str, args: &[&str]) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            shell: shell.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: None,
            env: BTreeMap::new(),
        }
    }

    /// Build the argv for this profile, optionally running a single command.
    pub fn argv(&self, command: Option<&str>) -> Vec<String> {
        let mut argv = vec![self.shell.clone()];
        argv.extend(self.args.iter().cloned());
        if let Some(c) = command {
            argv.push("-c".to_string());
            argv.push(c.to_string());
        }
        argv
    }
}

/// The built-in profile set, aligned with the AI profiles the installer offers.
pub fn builtin_profiles() -> Vec<Profile> {
    vec![
        Profile::new("shell", "Default Shell", "/bin/bash", &["--login", "-i"]),
        Profile::new("hcs", "HCS Command Line", "/usr/bin/hcs", &[]),
        Profile::new(
            "edge",
            "AI Profile: EDGE-8GB",
            "/usr/bin/hcs",
            &["model", "status"],
        ),
        Profile::new(
            "lowram",
            "AI Profile: LOWRAM-4GB",
            "/usr/bin/hcs",
            &["model", "status"],
        ),
        Profile::new(
            "workstation",
            "AI Profile: WORKSTATION-16GB",
            "/usr/bin/hcs",
            &["model", "status"],
        ),
        Profile::new("rust", "Rust", "/bin/bash", &["-c", "cd \"${PWD:-$HOME}\""]),
        Profile::new("python", "Python 3.12", "/bin/bash", &["-c", "python3"]),
        Profile::new("recovery", "Recovery Console", "/bin/bash", &["--login"]),
    ]
}

/// Rank profiles against a query for the `Alt+,` picker. Same determinism
/// contract as the action registry: title prefix beats substring, ties break on
/// id.
pub fn search_profiles<'a>(profiles: &'a [Profile], query: &str) -> Vec<&'a Profile> {
    let q = query.trim().to_lowercase();
    let mut scored: Vec<(i32, &'a Profile)> = profiles
        .iter()
        .filter_map(|p| {
            if q.is_empty() {
                return Some((0, p));
            }
            let n = p.name.to_lowercase();
            let i = p.id.to_lowercase();
            if n == q {
                Some((100, p))
            } else if n.starts_with(&q) {
                Some((90, p))
            } else if n.contains(&q) {
                Some((70, p))
            } else if i.starts_with(&q) {
                Some((60, p))
            } else if p.shell.to_lowercase().contains(&q) {
                Some((40, p))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    scored.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_text_at_cursor() {
        let mut g = Grid::new(20, 3);
        g.write_str("hello");
        assert_eq!(g.row_text(0), "hello");
        assert_eq!(g.cursor(), (0, 5));
    }

    #[test]
    fn wraps_at_right_margin() {
        let mut g = Grid::new(5, 3);
        g.write_str("abcdefgh");
        assert_eq!(g.row_text(0), "abcde");
        assert_eq!(g.row_text(1), "fgh");
    }

    #[test]
    fn scrolls_when_exceeding_rows() {
        let mut g = Grid::new(10, 2);
        g.write_str("one\ntwo\nthree");
        assert_eq!(g.row_text(0), "two");
        assert_eq!(g.row_text(1), "three");
    }

    #[test]
    fn carriage_return_overwrites_same_row() {
        let mut g = Grid::new(20, 2);
        g.write_str("aaa\rbbb");
        assert_eq!(g.row_text(0), "bbb");
    }

    #[test]
    fn tab_advances_to_the_next_eight_column_stop() {
        let mut g = Grid::new(40, 2);
        // "a" occupies column 0, so the next stop after it is column 8. "b" then
        // lands on 8, leaving the cursor at 9. The classic off-by-one here is
        // treating the cursor position itself as the stop.
        g.write_str("a\tb");
        assert_eq!(g.cursor().1, 9);
        assert_eq!(g.row_text(0).chars().nth(8), Some('b'));
    }

    #[test]
    fn tab_from_column_zero_lands_on_column_eight() {
        let mut g = Grid::new(40, 2);
        g.write_str("\tb");
        assert_eq!(g.cursor().1, 9);
    }

    #[test]
    fn ansi_colour_is_applied_and_resettable() {
        let mut g = Grid::new(10, 1);
        g.feed(b"\x1b[31mR\x1b[0mN");
        let red = g.cell(0, 0);
        assert_eq!(red.ch, 'R');
        assert_eq!(red.fg, Some(0xF8_51_49));
        let normal = g.cell(0, 1);
        assert_eq!(normal.ch, 'N');
        assert_eq!(normal.fg, None);
    }

    #[test]
    fn bold_sgr_sets_attribute() {
        let mut g = Grid::new(10, 1);
        g.feed(b"\x1b[1mB");
        assert!(g.cell(0, 0).bold);
    }

    #[test]
    fn erase_display_clears_the_grid() {
        let mut g = Grid::new(10, 2);
        g.write_str("stuff\nmore");
        g.feed(b"\x1b[2J");
        assert_eq!(g.text(), "");
    }

    #[test]
    fn unknown_escape_does_not_corrupt_output() {
        let mut g = Grid::new(20, 2);
        g.feed(b"a\x1b[?25lb");
        assert_eq!(g.row_text(0), "ab");
    }

    #[test]
    fn cell_out_of_range_is_blank_not_panic() {
        let g = Grid::new(4, 2);
        assert_eq!(g.cell(99, 99), Cell::blank());
    }

    #[test]
    fn grid_is_clamped_to_a_usable_minimum() {
        let g = Grid::new(0, 0);
        assert!(g.cols() >= 2);
        assert!(g.rows() >= 1);
    }

    #[test]
    fn profile_argv_includes_command() {
        let p = Profile::new("shell", "Shell", "/bin/bash", &["-i"]);
        let argv = p.argv(Some("echo hi"));
        assert_eq!(argv, vec!["/bin/bash", "-i", "-c", "echo hi"]);
    }

    #[test]
    fn profile_argv_without_command_omits_c_flag() {
        let p = Profile::new("shell", "Shell", "/bin/bash", &["-i"]);
        assert_eq!(p.argv(None), vec!["/bin/bash", "-i"]);
    }

    #[test]
    fn profile_search_prefix_beats_substring() {
        let profiles = builtin_profiles();
        let hits = search_profiles(&profiles, "AI Prof");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "edge");
    }

    #[test]
    fn profile_search_matches_id() {
        let profiles = builtin_profiles();
        let hits = search_profiles(&profiles, "lowram");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "lowram");
    }

    #[test]
    fn profile_search_empty_query_returns_all_sorted() {
        let profiles = builtin_profiles();
        let hits = search_profiles(&profiles, "");
        assert_eq!(hits.len(), profiles.len());
    }
}
