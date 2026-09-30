//! RAG over the offline manuals.
//!
//! v2 gap (B-15): `hcs-rag-ingest` existed as a library with no UI, no
//! evaluation and no reachable surface. This module is the retrieval side, and
//! it has one hard requirement borrowed from every honest RAG system: **an
//! answer must cite the section it came from**. An uncited answer is treated as
//! a failure, not as a slightly worse success.
//!
//! It is offline by construction — no network call exists in this file.

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Where the manuals live, on device and in a checkout.
pub fn manuals_dir() -> PathBuf {
    if let Ok(d) = std::env::var("HCS_MANUALS_DIR") {
        return PathBuf::from(d);
    }
    let on_device = PathBuf::from("/usr/share/hcs/docs/manuals");
    if on_device.is_dir() {
        on_device
    } else {
        PathBuf::from("config/includes.chroot/usr/share/hcs/docs/manuals")
    }
}

/// One indexed chunk: a heading-delimited section of a manual.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub source: String,
    pub heading: String,
    pub body: String,
}

impl Chunk {
    fn text(&self) -> String {
        format!("{}\n{}", self.heading, self.body).to_lowercase()
    }
}

/// Split a manual at its headings. Heading-level granularity is what makes a
/// citation useful: "see 04_security_pentest#HITL" beats "see the manual".
pub fn chunk_markdown(source: &str, body: &str) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut heading = String::from("(intro)");
    let mut buf: Vec<&str> = Vec::new();

    let flush = |heading: &mut String, buf: &mut Vec<&str>, out: &mut Vec<Chunk>| {
        if buf.is_empty() {
            return;
        }
        out.push(Chunk {
            source: source.to_string(),
            heading: std::mem::take(heading),
            body: buf.join("\n").trim().to_string(),
        });
        buf.clear();
    };

    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            flush(&mut heading, &mut buf, &mut out);
            heading = t.trim_start_matches('#').trim().to_string();
        } else {
            buf.push(line);
        }
    }
    flush(&mut heading, &mut buf, &mut out);
    out
}

/// Load and chunk every manual.
pub fn load_corpus() -> Vec<Chunk> {
    let dir = manuals_dir();
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    // Sorted so the corpus and therefore every index hash is reproducible.
    files.sort();
    for path in files {
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Ok(body) = std::fs::read_to_string(&path) {
            out.extend(chunk_markdown(&name, &body));
        }
    }
    out
}

/// One scored retrieval result.
#[derive(Debug, Clone)]
pub struct Hit {
    pub source: String,
    pub heading: String,
    pub score: f64,
    pub excerpt: String,
}

impl Hit {
    /// The citation string. Always present on a successful answer — that is the
    /// whole point.
    pub fn cite(&self) -> String {
        format!("{}.md#{}", self.source, slug(&self.heading))
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Rank chunks against a question with IDF-weighted term overlap.
///
/// IDF matters: without it a question about "the manual" ranks every chunk
/// equally and the answer becomes arbitrary.
/// Words that carry no retrieval signal.
///
/// Without this, "how does the kill switch work" ranks by "how", "does" and
/// "the" — all of which appear in nearly every chunk, so IDF alone cannot
/// suppress them reliably and the top hit becomes whichever section happens to
/// repeat a filler word.
const STOPWORDS: [&str; 24] = [
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "can", "do", "does", "for", "how", "i",
    "in", "is", "it", "of", "on", "or", "that", "the", "to",
];

fn is_stopword(t: &str) -> bool {
    STOPWORDS.contains(&t)
}

pub fn retrieve(chunks: &[Chunk], question: &str, limit: usize) -> Vec<Hit> {
    // Punctuation is stripped so "kill-switch" and "kill switch" match.
    let raw: Vec<String> = question
        .split_whitespace()
        .map(|t| {
            t.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|t: &String| !t.is_empty())
        .collect();
    if raw.is_empty() {
        return Vec::new();
    }

    let filtered: Vec<String> = raw.iter().filter(|t| !is_stopword(t)).cloned().collect();

    // Search with the content words first. If that finds nothing — which happens
    // when the query is *only* filler, or uses words we do not know — fall back to
    // the unfiltered terms rather than reporting an empty result. An empty result
    // would tell the user their question was unanswerable when in fact we simply
    // filtered out everything they said.
    let primary = score_chunks(chunks, &filtered, limit);
    if !primary.is_empty() || filtered.is_empty() {
        return primary;
    }
    score_chunks(chunks, &raw, limit)
}

fn score_chunks(chunks: &[Chunk], terms: &[String], limit: usize) -> Vec<Hit> {
    if terms.is_empty() {
        return Vec::new();
    }

    let texts: Vec<String> = chunks.iter().map(|c| c.text()).collect();
    // Headings are indexed separately and weighted far higher. A query whose
    // words *are* a section title is asking for that section: without this
    // boost, a passage that merely mentions "kill switch" twice outranks the
    // section actually titled "The kill switch".
    let headings: Vec<String> = chunks.iter().map(|c| c.heading.to_lowercase()).collect();
    const HEADING_BOOST: f64 = 3.0;
    const FILENAME_BOOST: f64 = 1.5;
    // The boost is applied per content term, not to the concatenated phrase: a
    // heading rarely contains the whole sentence, but it almost always contains
    // the distinctive words the user typed.
    let phrase = terms.join(" ");

    let mut hits: Vec<Hit> = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let mut score = 0.0;
        for t in terms.iter() {
            if !texts[i].contains(t.as_str()) {
                continue;
            }
            let df = texts
                .iter()
                .filter(|x| x.contains(t.as_str()))
                .count()
                .max(1);
            let idf = (chunks.len() as f64 / df as f64).ln().max(0.1);
            let tf = texts[i].matches(t.as_str()).count() as f64;
            score += idf * (1.0 + tf.ln());
        }
        for t in terms.iter() {
            if headings[i].contains(t.as_str()) {
                score += HEADING_BOOST;
            }
        }
        if chunk.source.to_lowercase().contains(&phrase) {
            score += FILENAME_BOOST;
        }
        if score > 0.0 {
            hits.push(Hit {
                source: chunk.source.clone(),
                heading: chunk.heading.clone(),
                score,
                excerpt: excerpt(&chunk.body, terms),
            });
        }
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.source.cmp(&b.source))
            .then_with(|| a.heading.cmp(&b.heading))
    });
    hits.truncate(limit);
    hits
}

/// Refuse to answer rather than guess. An ungrounded answer about a security
/// feature is worse than "I do not know".
pub fn refuses(question: &str) -> bool {
    let q = question.to_lowercase();
    if q.trim().is_empty() {
        return true;
    }
    // Questions about something outside the documented scope.
    const OUT_OF_SCOPE: [&str; 5] = ["weather", "stock price", "bitcoin", "recipe", "who won"];
    OUT_OF_SCOPE.iter().any(|t| q.contains(t))
}

fn excerpt(body: &str, terms: &[String]) -> String {
    let lower = body.to_lowercase();
    let pos = terms
        .iter()
        .filter_map(|t| lower.find(t.as_str()))
        .min()
        .unwrap_or(0);
    let chars: Vec<char> = body.chars().collect();
    let start = pos.saturating_sub(60);
    let end = (pos + 160).min(chars.len());
    let mut out: String = chars[start..end].iter().collect();
    if start > 0 {
        out.insert(0, '…');
    }
    if end < chars.len() {
        out.push('…');
    }
    out
}

/// `hcs rag query` implementation.
pub fn query(question: &str) -> Result<()> {
    if refuses(question) {
        println!("[REFUSED] I do not know that from the offline manuals.");
        println!(
            "This system only answers from {} and its indexed documents.",
            manuals_dir().display()
        );
        println!("An uncited answer would be a guess, and a guess about a security");
        println!("feature is worse than no answer.");
        return Ok(());
    }

    let chunks = load_corpus();
    if chunks.is_empty() {
        println!("[EMPTY] No manuals found at {}.", manuals_dir().display());
        println!("Run `hcs rag rebuild` after installing the documentation.");
        return Ok(());
    }

    let hits = retrieve(&chunks, question, 3);
    if hits.is_empty() {
        println!("[NO MATCH] Nothing in the manuals matches '{question}'.");
        println!(
            "{} chunks were searched. Refusing to invent an answer.",
            chunks.len()
        );
        return Ok(());
    }

    println!("Question: {question}");
    println!();
    for (i, h) in hits.iter().enumerate() {
        println!("[{}] {} (score {:.2})", i + 1, h.cite(), h.score);
        println!("    {}", h.excerpt);
        println!();
    }
    println!(
        "Sources: {}",
        hits.iter().map(Hit::cite).collect::<Vec<_>>().join(", ")
    );
    Ok(())
}

pub fn stats() {
    let chunks = load_corpus();
    let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
    for c in &chunks {
        *by_source.entry(c.source.clone()).or_insert(0) += 1;
    }
    println!("RAG index over the offline manuals");
    println!("  corpus:    {}", manuals_dir().display());
    println!("  chunks:    {}", chunks.len());
    for (src, n) in &by_source {
        println!("    {:<28} {n} chunk(s)", format!("{src}.md"));
    }
    println!("  backend:   lexical IDF over heading-delimited chunks");
    println!("  network:   none — retrieval is entirely offline");
    println!("  citations: mandatory; an uncited answer counts as a failure");
}

pub fn rebuild() {
    let chunks = load_corpus();
    println!(
        "[OK] Indexed {} chunk(s) from {}.",
        chunks.len(),
        manuals_dir().display()
    );
    println!("An embedding index can be built once a resident model is available;");
    println!("lexical retrieval is used until then so the feature works offline out of the box.");
}

/// Explain whatever is on screen (iOS 26 / macOS 27 Visual Intelligence
/// pattern): the selection is captured, matched against the manuals, and
/// answered with citations — or refused.
pub fn explain_selection(captured: &str) -> Result<()> {
    if captured.trim().is_empty() {
        println!("[EMPTY] Nothing could be read from the current selection.");
        println!("Select some text first, or ask a question directly instead.");
        return Ok(());
    }
    println!("HCS Brain — explaining the current selection");
    println!();
    let preview: String = captured.chars().take(400).collect();
    println!("Selection:\n  {preview}");
    println!();

    let chunks = load_corpus();
    let hits = retrieve(&chunks, captured, 2);
    if hits.is_empty() {
        println!("[NO MATCH] The manuals do not cover this selection.");
        println!(
            "{} chunks searched; refusing to invent an explanation.",
            chunks.len()
        );
        return Ok(());
    }
    for h in &hits {
        println!("[{}] (score {:.2})", h.cite(), h.score);
        println!("    {}", h.excerpt);
        println!();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Vec<Chunk> {
        vec![
            Chunk {
                source: "01_getting_started".into(),
                heading: "First boot".into(),
                body: "Boot from the USB stick. The live session starts automatically.".into(),
            },
            Chunk {
                source: "04_security_pentest".into(),
                heading: "HITL gate".into(),
                body: "Every pentester action requires human confirmation before it runs.".into(),
            },
            Chunk {
                source: "05_tor_anonymity".into(),
                heading: "Kill switch".into(),
                body: "The nftables rules fail closed: traffic without Tor is dropped, so zero DNS leaks."
                    .into(),
            },
        ]
    }

    #[test]
    fn chunking_splits_at_headings() {
        let chunks = chunk_markdown("doc", "# One\nalpha\n## Two\nbeta");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].heading, "One");
        assert!(chunks[0].body.contains("alpha"));
        assert_eq!(chunks[1].heading, "Two");
        assert!(chunks[1].body.contains("beta"));
    }

    #[test]
    fn a_manual_without_headings_yields_one_chunk() {
        let chunks = chunk_markdown("doc", "just text\nmore text");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].heading, "(intro)");
    }

    #[test]
    fn retrieval_finds_the_right_section() {
        let hits = retrieve(&corpus(), "how does the kill switch work", 3);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].source, "05_tor_anonymity");
    }

    #[test]
    fn filler_words_do_not_dominate_the_ranking() {
        // "how", "does" and "the" appear in nearly every chunk, so without
        // stopword removal the top hit becomes whichever section repeats a
        // filler word rather than the one about the kill switch.
        let c = vec![
            Chunk {
                source: "a".into(),
                heading: "How to install".into(),
                body: "how does this work how do i install it how is it done".into(),
            },
            Chunk {
                source: "b".into(),
                heading: "The kill switch".into(),
                body: "the kill switch drops traffic that is not proxied.".into(),
            },
        ];
        let hits = retrieve(&c, "how does the kill switch work", 3);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].source, "b", "filler words must not win");
    }

    #[test]
    fn a_query_of_only_filler_still_returns_something() {
        let hits = retrieve(&corpus(), "what is the", 3);
        assert!(
            !hits.is_empty(),
            "stripping every term would silently return nothing, which reads as a bug"
        );
    }

    #[test]
    fn a_section_title_outranks_a_passage_that_merely_mentions_it() {
        let c = vec![
            Chunk {
                source: "chat".into(),
                heading: "Asking questions".into(),
                body: "you can also ask about the kill switch if you like".into(),
            },
            Chunk {
                source: "tor".into(),
                heading: "The kill switch".into(),
                body: "traffic without Tor is dropped.".into(),
            },
        ];
        let hits = retrieve(&c, "kill switch", 3);
        assert_eq!(hits[0].source, "tor");
    }

    #[test]
    fn rare_terms_outrank_common_ones() {
        let c = vec![
            Chunk {
                source: "a".into(),
                heading: "h".into(),
                body: "boot boot boot boot".into(),
            },
            Chunk {
                source: "b".into(),
                heading: "h".into(),
                body: "boot and plymouth and calamares and squashfs".into(),
            },
        ];
        let hits = retrieve(&c, "plymouth", 3);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source, "b");
    }

    #[test]
    fn every_hit_carries_a_citation() {
        for h in retrieve(&corpus(), "kill switch", 3) {
            let c = h.cite();
            // A citation must name *both* the manual and the section. A bare
            // "file.md#" is worse than no citation, because it looks precise.
            let (file, section) = c
                .split_once('#')
                .unwrap_or_else(|| panic!("citation has no section anchor: {c}"));
            assert!(file.ends_with(".md"), "citation must name a manual: {c}");
            assert!(!section.is_empty(), "citation must name a section: {c}");
            assert!(
                section
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '-'),
                "section anchor must be a slug: {c}"
            );
        }
    }

    #[test]
    fn citation_slug_is_url_safe() {
        let h = Hit {
            source: "05_tor_anonymity".into(),
            heading: "Kill switch / DNS leaks".into(),
            score: 1.0,
            excerpt: String::new(),
        };
        assert_eq!(h.cite(), "05_tor_anonymity.md#kill-switch-dns-leaks");
    }

    #[test]
    fn out_of_scope_questions_are_refused() {
        for q in [
            "what is the weather",
            "what is the bitcoin price",
            "who won the match",
        ] {
            assert!(refuses(q), "{q} must be refused");
        }
    }

    #[test]
    fn in_scope_questions_are_not_refused() {
        assert!(!refuses("how does the Tor kill switch work"));
        assert!(!refuses("what is the RAM budget"));
    }

    #[test]
    fn empty_question_is_refused() {
        assert!(refuses("   "));
    }

    #[test]
    fn unmatched_question_yields_no_hits_rather_than_a_guess() {
        assert!(retrieve(&corpus(), "quantum chromodynamics", 3).is_empty());
    }

    #[test]
    fn retrieval_order_is_deterministic() {
        let c = corpus();
        let a: Vec<String> = retrieve(&c, "gate", 3).iter().map(Hit::cite).collect();
        let b: Vec<String> = retrieve(&c, "gate", 3).iter().map(Hit::cite).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn load_corpus_is_reproducible() {
        let a = load_corpus();
        let b = load_corpus();
        let names_a: Vec<&str> = a.iter().map(|c| c.source.as_str()).collect();
        let names_b: Vec<&str> = b.iter().map(|c| c.source.as_str()).collect();
        assert_eq!(
            names_a, names_b,
            "chunk order must be stable for a stable index hash"
        );
    }
}
