//! HCS Recall — a local, encrypted timeline of what you have seen.
//!
//! Pattern: Windows 11 Recall. Three properties of it are worth copying, and
//! one is worth refusing:
//!
//!   COPY: a *timeline* plus **semantic** search, so "that recipe I saw last
//!         week" is findable by description rather than by filename.
//!   COPY: a **sensitive-information filter** that excludes anything matching
//!         credential and payment patterns before a snapshot is ever written.
//!   COPY: the feature is **opt-in**, and paused automatically when free space
//!         runs low.
//!   REFUSE: Recall uploads snapshots to a vendor. Everything here stays on the
//!         machine, and in amnesic mode the store refuses to exist at all.

use serde::{Deserialize, Serialize};
use std::fmt;

/// One captured frame of activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    /// Unix seconds.
    pub captured: i64,
    pub app: String,
    pub window_title: String,
    /// Extracted text for search. Empty for frames where OCR found nothing.
    pub text: String,
    /// Path of the encrypted image, relative to the store root.
    pub image: String,
    /// Bytes on disk.
    pub size_bytes: u64,
}

/// Why a candidate snapshot was not written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterReason {
    /// Matched a credential pattern.
    Credential,
    /// Matched a payment-card pattern.
    Payment,
    /// The app is on the exclusion list (a password manager, a banking app).
    ExcludedApp,
    /// Store is full or below the free-space floor.
    LowSpace,
}

impl fmt::Display for FilterReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FilterReason::Credential => "matched a credential pattern",
            FilterReason::Payment => "matched a payment-card pattern",
            FilterReason::ExcludedApp => "belongs to an excluded application",
            FilterReason::LowSpace => "free space is below the floor",
        };
        write!(f, "{s}")
    }
}

/// The outcome of inspecting a frame before it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Store,
    Skip(FilterReason),
}

/// Patterns that must never reach the store.
///
/// These are deliberately conservative and substring-based. A false positive
/// costs the user one missed frame; a false negative puts a password into an
/// index that was supposed to be safe. The asymmetry decides the trade.
pub fn screen(text: &str, app: &str, excluded_apps: &[&str]) -> Decision {
    let lower = text.to_lowercase();

    const CREDENTIAL_MARKERS: [&str; 8] = [
        "password",
        "passwort",
        "api key",
        "api-key",
        "apikey",
        "secret key",
        "private key",
        "bearer ",
    ];
    for m in CREDENTIAL_MARKERS {
        if lower.contains(m) {
            return Decision::Skip(FilterReason::Credential);
        }
    }

    // Card-number shapes: 13-19 digits with optional separators.
    if has_card_number(text) {
        return Decision::Skip(FilterReason::Payment);
    }

    if excluded_apps.iter().any(|a| a.eq_ignore_ascii_case(app)) {
        return Decision::Skip(FilterReason::ExcludedApp);
    }

    Decision::Store
}

/// Detect a card-number shape: 13–19 digits, optionally grouped by a *single*
/// consistent separator (spaces or hyphens), confirmed with a Luhn check.
///
/// Grouping is read from maximal runs rather than from the single longest
/// block, because "4111 1111 1111 1111" has four four-digit groups but is one
/// sixteen-digit card. Requiring one consistent separator keeps a timestamp
/// sitting next to an unrelated long number from being joined into one run.
fn has_card_number(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut digits = String::new();
        let mut separator: Option<char> = None;
        while i < chars.len() {
            if chars[i].is_ascii_digit() {
                digits.push(chars[i]);
                i += 1;
                continue;
            }
            let sep = chars[i];
            if sep == ' ' || sep == '-' {
                // A second, different separator ends the run.
                if separator.is_some() && separator != Some(sep) {
                    break;
                }
                separator = Some(sep);
                i += 1;
                continue;
            }
            break;
        }
        if (13..=19).contains(&digits.len()) && luhn_ok(&digits) {
            return true;
        }
    }
    false
}

fn luhn_ok(digits: &str) -> bool {
    let mut sum = 0u32;
    for (i, c) in digits.chars().rev().enumerate() {
        let mut d = c.to_digit(10).unwrap_or(0);
        if i % 2 == 1 {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
    }
    sum.is_multiple_of(10)
}

/// Free-space floor, borrowed from Recall's own behaviour of pausing below a
/// threshold. Below this, capture pauses rather than filling the disk.
pub const FREE_SPACE_FLOOR_GB: u64 = 25;

/// Whether the store may accept a new snapshot given free disk space.
pub fn space_allows(free_gb: u64) -> Decision {
    if free_gb < FREE_SPACE_FLOOR_GB {
        Decision::Skip(FilterReason::LowSpace)
    } else {
        Decision::Store
    }
}

/// A scored search hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub score: f64,
    pub app: String,
    pub captured: i64,
    pub excerpt: String,
}

/// Search the timeline. Semantic-ish rather than truly semantic: we score on
/// token overlap with an IDF-style boost for rare terms, which is enough to
/// make "that thing about the tor bridge" find the right frame without a model
/// and without a network call.
pub fn search(snapshots: &[Snapshot], query: &str, limit: usize) -> Vec<Hit> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| {
            t.to_lowercase()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_string()
        })
        .filter(|t| !t.is_empty())
        .collect();
    if terms.is_empty() {
        return Vec::new();
    }

    // Document frequency, for the IDF boost.
    let mut hits: Vec<Hit> = snapshots
        .iter()
        .filter_map(|s| {
            let hay = format!("{} {} {}", s.app, s.window_title, s.text).to_lowercase();
            let mut score = 0.0;
            for t in &terms {
                let occurrences = hay.matches(t.as_str()).count();
                if occurrences == 0 {
                    continue;
                }
                let df = snapshots
                    .iter()
                    .filter(|o| {
                        format!("{} {} {}", o.app, o.window_title, o.text)
                            .to_lowercase()
                            .contains(t.as_str())
                    })
                    .count()
                    .max(1);
                let idf = (snapshots.len() as f64 / df as f64).ln().max(0.1);
                // Saturating term frequency: a frame that mentions the term four
                // times is more likely the one meant, but the fifth mention adds
                // far less than the first. A linear count would let a single
                // repetitive frame dominate every query.
                let tf = (1.0 + (occurrences as f64).ln()).min(3.0);
                score += idf * tf;
            }
            if score <= 0.0 {
                return None;
            }
            Some(Hit {
                id: s.id.clone(),
                score,
                app: s.app.clone(),
                captured: s.captured,
                excerpt: excerpt(&s.text, &terms),
            })
        })
        .collect();

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.captured.cmp(&a.captured))
            .then_with(|| a.id.cmp(&b.id))
    });
    hits.truncate(limit);
    hits
}

/// Build a short excerpt around the first matching term.
pub fn excerpt(text: &str, terms: &[String]) -> String {
    let lower = text.to_lowercase();
    let pos = terms
        .iter()
        .filter_map(|t| lower.find(t.as_str()))
        .min()
        .unwrap_or(0);
    let chars: Vec<char> = text.chars().collect();
    let start = pos.saturating_sub(30);
    let end = (pos + 70).min(chars.len());
    if chars.is_empty() {
        return String::new();
    }
    let mut out: String = chars[start..end].iter().collect();
    if start > 0 {
        out.insert(0, '…');
    }
    if end < chars.len() {
        out.push('…');
    }
    out
}

/// Whether the store may exist at all in the current session.
///
/// In an amnesic session Recall has nothing to offer and everything to cost, so
/// it refuses rather than silently recording nothing.
pub fn allowed_in_session(amnesic: bool, user_opted_in: bool) -> bool {
    user_opted_in && !amnesic
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXCLUDED: [&str; 3] = ["hcs-vault", "gnome-passwords", "keepassxc"];

    fn snap(id: &str, app: &str, title: &str, text: &str, at: i64) -> Snapshot {
        Snapshot {
            id: id.into(),
            captured: at,
            app: app.into(),
            window_title: title.into(),
            text: text.into(),
            image: format!("{id}.enc"),
            size_bytes: 1024,
        }
    }

    #[test]
    fn ordinary_screen_is_stored() {
        assert_eq!(
            screen("Shopping list: milk, coffee", "firefox", &EXCLUDED),
            Decision::Store
        );
    }

    #[test]
    fn password_label_is_filtered() {
        assert_eq!(
            screen("Password: hunter2", "firefox", &EXCLUDED),
            Decision::Skip(FilterReason::Credential)
        );
    }

    #[test]
    fn german_passwort_is_also_filtered() {
        assert_eq!(
            screen("Passwort: geheim", "firefox", &EXCLUDED),
            Decision::Skip(FilterReason::Credential)
        );
    }

    #[test]
    fn api_key_is_filtered() {
        assert_eq!(
            screen("API key: sk-abc123", "codium", &EXCLUDED),
            Decision::Skip(FilterReason::Credential)
        );
    }

    #[test]
    fn bearer_token_is_filtered() {
        assert_eq!(
            screen("Authorization: Bearer abc.def", "firefox", &EXCLUDED),
            Decision::Skip(FilterReason::Credential)
        );
    }

    #[test]
    fn excluded_app_is_skipped_even_with_clean_text() {
        assert_eq!(
            screen("nothing sensitive here", "HCS-Vault", &EXCLUDED),
            Decision::Skip(FilterReason::ExcludedApp)
        );
    }

    #[test]
    fn valid_card_number_is_filtered() {
        // Luhn-valid test number from the standard test set.
        assert_eq!(
            screen("card 4111 1111 1111 1111", "firefox", &EXCLUDED),
            Decision::Skip(FilterReason::Payment)
        );
    }

    #[test]
    fn long_number_failing_luhn_is_not_treated_as_a_card() {
        assert_eq!(
            screen("order 1234567890123456 shipped", "firefox", &EXCLUDED),
            Decision::Store
        );
    }

    #[test]
    fn short_numbers_are_never_cards() {
        assert_eq!(
            screen("build 1234567890 done", "codium", &EXCLUDED),
            Decision::Store
        );
    }

    #[test]
    fn luhn_accepts_a_known_good_number() {
        assert!(luhn_ok("4111111111111111"));
    }

    #[test]
    fn luhn_rejects_a_bad_number() {
        assert!(!luhn_ok("4111111111111112"));
    }

    #[test]
    fn low_space_pauses_capture() {
        assert_eq!(
            space_allows(FREE_SPACE_FLOOR_GB - 1),
            Decision::Skip(FilterReason::LowSpace)
        );
        assert_eq!(space_allows(FREE_SPACE_FLOOR_GB), Decision::Store);
    }

    #[test]
    fn recall_requires_opt_in() {
        assert!(!allowed_in_session(false, false));
        assert!(allowed_in_session(false, true));
    }

    #[test]
    fn recall_refuses_to_exist_in_an_amnesic_session() {
        assert!(
            !allowed_in_session(true, true),
            "amnesic mode must win over an explicit opt-in"
        );
    }

    #[test]
    fn search_finds_by_content() {
        let snaps = vec![
            snap(
                "s1",
                "firefox",
                "Bridge settings",
                "tor bridge obfs4 configuration",
                100,
            ),
            snap("s2", "codium", "main.rs", "fn main() { println!() }", 200),
        ];
        let hits = search(&snaps, "tor bridge", 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "s1");
    }

    #[test]
    fn search_finds_by_app_name() {
        let snaps = vec![snap("s1", "firefox", "Docs", "some text", 100)];
        assert_eq!(search(&snaps, "firefox", 5).len(), 1);
    }

    #[test]
    fn rare_terms_outrank_common_ones() {
        let snaps = vec![
            snap(
                "s1",
                "a",
                "",
                "kernel module signing and secure boot policy",
                100,
            ),
            snap(
                "s2",
                "b",
                "",
                "module one module two module three module four",
                200,
            ),
        ];
        let hits = search(&snaps, "secure boot", 5);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "s1");
    }

    #[test]
    fn search_with_empty_query_returns_nothing() {
        let snaps = vec![snap("s1", "a", "", "text", 1)];
        assert!(search(&snaps, "   ", 5).is_empty());
    }

    #[test]
    fn search_respects_the_limit_and_sorts_by_score() {
        let snaps = vec![
            snap("s1", "a", "", "tor tor tor tor", 100),
            snap("s2", "b", "", "tor and other words entirely here", 200),
        ];
        let hits = search(&snaps, "tor", 1);
        assert_eq!(hits.len(), 1);
        // The frame matching in more places scores higher.
        assert_eq!(hits[0].id, "s1");
    }

    #[test]
    fn excerpt_centres_on_the_match() {
        let text = "a".repeat(50) + " TARGET " + &"b".repeat(200);
        let e = excerpt(&text, &["target".to_string()]);
        assert!(e.contains("TARGET"));
        assert!(e.starts_with('…'));
    }

    #[test]
    fn excerpt_of_empty_text_is_empty() {
        assert_eq!(excerpt("", &["x".to_string()]), "");
    }

    #[test]
    fn filter_reasons_explain_themselves() {
        assert!(FilterReason::Credential.to_string().contains("credential"));
        assert!(FilterReason::LowSpace.to_string().contains("free space"));
    }
}
