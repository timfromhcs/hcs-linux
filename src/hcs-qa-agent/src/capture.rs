//! Capture analysis — deciding whether a screenshot is real evidence.
//!
//! v1 scored VM captures with Shannon entropy alone, and that gate passed three
//! real rendering bugs: a solid fill still reaches ~1.6 entropy after PNG
//! quantisation, so entropy cannot tell "a UI drew" from "one colour was
//! painted". What actually distinguishes them:
//!
//! * **unique colours** — a rendered window uses many; a solid fill uses one.
//! * **text pixels** — a share of pixels that differ from the modal colour. A
//!   window with chrome and labels always has a meaningful minority; a fill
//!   has essentially none.
//! * **file size** — a truncated or empty capture is tiny.
//!
//! The thresholds live here, in one place, as pure functions over decoded
//! pixels, because the version of this check that shipped lived in a PowerShell
//! heredoc where no test could reach it. It runs identically in the guest (so
//! the image can judge its own frames) and on the host (so the evidence is
//! re-checked after transfer, not trusted).

use serde::{Deserialize, Serialize};

/// The v2 capture thresholds. Tightened relative to v1: 0.5 entropy and 8
/// colours passed solid fills.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    /// Smallest acceptable PNG size in bytes.
    pub min_bytes: u64,
    /// Smallest acceptable width in pixels.
    pub min_width: u32,
    /// Smallest acceptable height in pixels.
    pub min_height: u32,
    /// Smallest acceptable number of distinct colours.
    pub min_unique_colors: usize,
    /// Smallest acceptable Shannon entropy, in bits per channel.
    pub min_entropy: f64,
    /// Smallest acceptable share of non-modal pixels. Catches a solid fill.
    pub min_text_ratio: f64,
    /// Smallest acceptable share of pixels with a strong local colour step.
    /// This is the gate that actually proves something was *drawn*.
    pub min_edge_density: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            min_bytes: 4096,
            min_width: 640,
            min_height: 400,
            min_unique_colors: 24,
            // NOTE the floor. Entropy is computed over three *separate* channel
            // histograms, so every image scores at least log2(3) = 1.585 — a
            // perfectly solid fill measures exactly that. Any threshold at or
            // below 1.585 is dead code; v1's 0.5 was, which is why a solid fill
            // passed it. The 72 committed renders measure 2.21 (chat/high
            // contrast, the sparsest) to 4.02, so 1.8 sits above the floor and
            // below the sparsest real UI.
            min_entropy: 1.8,
            // 0.3% of pixels. A 1280x800 frame is ~1M pixels, so this is ~3000
            // pixels of text and icons. v1 used nothing at all here, which is
            // why a window with a solid title bar passed as "text rendered".
            min_text_ratio: 0.003,
            // Measured, not guessed. The 72 committed reference renders score
            // 0.041 (chat/stealth, the sparsest) to 0.097 (image studio), and a
            // smooth gradient — a wallpaper or a plymouth splash — scores 0.
            // 0.010 sits an order of magnitude clear of the gradient and still
            // four times below the sparsest real UI, so it fails pictures
            // without being tight enough to catch a legitimately sparse window.
            min_edge_density: 0.010,
        }
    }
}

/// Per-channel difference that counts as an edge for `edge_density`.
///
/// Well below the contrast of rendered text (usually 100+) and well above the
/// single-digit step of a smooth gradient.
pub const EDGE_STEP: u8 = 24;

/// What the analysis found. Serialised into the report so a reader can see the
/// numbers behind a verdict rather than trusting a boolean.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureReport {
    pub file: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub unique_colors: usize,
    pub entropy: f64,
    pub text_ratio: f64,
    pub edge_density: f64,
    pub ok: bool,
    /// Empty when `ok`; otherwise the first gate that failed, phrased so a
    /// human can act on it.
    pub reason: String,
}

/// A decoded image, kept minimal so the analysis does not depend on an image
/// decoder: the caller supplies pixels.
pub struct Pixels<'a> {
    pub width: u32,
    pub height: u32,
    /// RGB triples, row-major, `width * height * 3` bytes.
    pub rgb: &'a [u8],
    pub bytes_on_disk: u64,
    pub file: String,
}

impl Pixels<'_> {
    pub fn new<'a>(
        file: &str,
        width: u32,
        height: u32,
        rgb: &'a [u8],
        bytes_on_disk: u64,
    ) -> Option<Pixels<'a>> {
        let need = width as usize * height as usize * 3;
        if rgb.len() < need {
            return None;
        }
        Some(Pixels {
            width,
            height,
            rgb,
            bytes_on_disk,
            file: file.to_string(),
        })
    }
}

/// Shannon entropy over the three channel histograms together, in bits.
pub fn entropy(rgb: &[u8]) -> f64 {
    let mut hist = [0u64; 256 * 3];
    for px in rgb.as_chunks::<3>().0 {
        hist[px[0] as usize] += 1;
        hist[256 + px[1] as usize] += 1;
        hist[512 + px[2] as usize] += 1;
    }
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let t = total as f64;
    let mut e = 0.0;
    for c in hist.iter().filter(|c| **c > 0) {
        let p = *c as f64 / t;
        e -= p * p.log2();
    }
    e
}

/// Number of distinct RGB triples, capped so a pathological image cannot make
/// this quadratic. Above the cap the count is reported as the cap, which is
/// already far past any threshold.
pub fn unique_colors(rgb: &[u8], cap: usize) -> usize {
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for px in rgb.as_chunks::<3>().0 {
        let key = ((px[0] as u32) << 16) | ((px[1] as u32) << 8) | px[2] as u32;
        seen.insert(key);
        if seen.len() > cap {
            return cap + 1;
        }
    }
    seen.len()
}

/// Share of pixels that are *not* the most common colour.
///
/// Catches a solid fill, and nothing else. A smooth gradient defeats it
/// entirely — every pixel is a different colour, so the ratio approaches 1.0 —
/// which is why it is not the text gate on its own.
pub fn text_ratio(rgb: &[u8]) -> f64 {
    let mut hist: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    let mut total = 0u64;
    for px in rgb.as_chunks::<3>().0 {
        let key = ((px[0] as u32) << 16) | ((px[1] as u32) << 8) | px[2] as u32;
        *hist.entry(key).or_insert(0) += 1;
        total += 1;
    }
    if total == 0 {
        return 0.0;
    }
    let modal = hist.values().copied().max().unwrap_or(0);
    1.0 - (modal as f64 / total as f64)
}

/// Share of pixels that sit next to a much brighter or darker pixel.
///
/// This is the gate that proves a *user interface* was rendered rather than a
/// picture. Text, window borders, buttons and icons all produce a strong local
/// step; a gradient wallpaper, a boot splash or a solid fill produce almost
/// none, no matter how many colours or how much entropy they carry.
///
/// `step` is the per-channel difference that counts as an edge. 24 is well below
/// the contrast of rendered text (usually 100+) and well above the per-pixel
/// step of a smooth gradient, which is single digits.
pub fn edge_density(rgb: &[u8], width: u32, height: u32, step: u8) -> f64 {
    if width < 2 || height < 2 {
        return 0.0;
    }
    let at = |x: u32, y: u32| -> (i16, i16, i16) {
        let i = ((y as usize) * width as usize + x as usize) * 3;
        (rgb[i] as i16, rgb[i + 1] as i16, rgb[i + 2] as i16)
    };
    let diff = |a: (i16, i16, i16), b: (i16, i16, i16)| -> i16 {
        (a.0 - b.0)
            .abs()
            .max((a.1 - b.1).abs())
            .max((a.2 - b.2).abs())
    };
    let s = step as i16;

    let mut edges = 0u64;
    let mut total = 0u64;
    for y in 0..height {
        for x in 0..width {
            total += 1;
            let p = at(x, y);
            let mut edge = false;
            if x + 1 < width && diff(p, at(x + 1, y)) >= s {
                edge = true;
            }
            if !edge && y + 1 < height && diff(p, at(x, y + 1)) >= s {
                edge = true;
            }
            if edge {
                edges += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        edges as f64 / total as f64
    }
}

/// Analyse one capture against the thresholds.
pub fn analyse(p: &Pixels<'_>, t: &Thresholds) -> CaptureReport {
    let mut r = CaptureReport {
        file: p.file.clone(),
        width: p.width,
        height: p.height,
        bytes: p.bytes_on_disk,
        unique_colors: unique_colors(p.rgb, 65536),
        entropy: entropy(p.rgb),
        text_ratio: text_ratio(p.rgb),
        edge_density: edge_density(p.rgb, p.width, p.height, EDGE_STEP),
        ok: true,
        reason: String::new(),
    };

    let fail = |r: &mut CaptureReport, why: String| {
        r.ok = false;
        if r.reason.is_empty() {
            r.reason = why;
        }
    };

    if p.bytes_on_disk < t.min_bytes {
        fail(
            &mut r,
            format!(
                "only {} bytes on disk (minimum {}) — truncated or empty capture",
                p.bytes_on_disk, t.min_bytes
            ),
        );
    }
    if p.width < t.min_width || p.height < t.min_height {
        fail(
            &mut r,
            format!(
                "{}x{} is smaller than the {}x{} minimum",
                p.width, p.height, t.min_width, t.min_height
            ),
        );
    }
    // The values are read into locals first: formatting borrows `r` immutably
    // while `fail` needs it mutably.
    let (colors, ent, ratio, edges) = (r.unique_colors, r.entropy, r.text_ratio, r.edge_density);
    if colors < t.min_unique_colors {
        fail(
            &mut r,
            format!(
                "only {colors} unique colours (minimum {}) — this is a solid fill, not a window",
                t.min_unique_colors
            ),
        );
    }
    if ent < t.min_entropy {
        fail(
            &mut r,
            format!(
                "entropy {ent:.3} below {:.3} — the frame carries almost no information",
                t.min_entropy
            ),
        );
    }
    if ratio < t.min_text_ratio {
        fail(
            &mut r,
            format!(
                "text ratio {ratio:.5} below {:.5} — the frame is a single flat fill",
                t.min_text_ratio
            ),
        );
    }
    if edges < t.min_edge_density {
        fail(
            &mut r,
            format!(
                "edge density {edges:.5} below {:.5} — nothing was drawn on the frame. \
                 A gradient, a splash or a wallpaper all have many colours and high \
                 entropy while showing no interface at all.",
                t.min_edge_density
            ),
        );
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(colour: [u8; 3], w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 3) as usize);
        for _ in 0..(w * h) {
            v.extend_from_slice(&colour);
        }
        v
    }

    /// A frame that looks like a window: a dark body, a title bar, card
    /// surfaces with borders, and rows of "text" with antialiased edges.
    ///
    /// Deliberately noisier than a flat mock: real renders have thousands of
    /// distinct colours from antialiasing and gradients, and a fixture with
    /// three colours would be testing a picture no renderer ever produces.
    fn window(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                let body = 14 + ((x / 7 + y / 5) % 6) as u8;
                let c = if y < h / 8 {
                    // Title bar with a subtle vertical shade.
                    let t = (y % 5) as u8;
                    [34u8 + t, 42u8 + (y % 7) as u8, 54u8 + (y % 9) as u8]
                } else if y > h - h / 6 && x > 24 && x < w - 24 {
                    // A card surface, slightly lighter than the body.
                    [body + 10, body + 13, body + 18]
                } else if y % 29 < 9 && x % 7 < 4 && x > 40 && x < w - 60 {
                    // Glyph rows, with an antialiasing ramp on the edge column.
                    let ramp = if x % 7 == 3 { 90 } else { 0 };
                    [225u8 - ramp, 235 - ramp, 245 - ramp]
                } else {
                    [body, body + 4, body + 12]
                };
                v.extend_from_slice(&c);
            }
        }
        v
    }

    #[test]
    fn a_solid_fill_is_rejected() {
        // This is the exact case v1's entropy-only gate passed: entropy over a
        // single-colour image is 0, but a *quantised* fill could reach ~1.6, and
        // more importantly the fill had no text at all.
        let px = solid([16, 20, 28], 1280, 800);
        let p = Pixels::new("solid.png", 1280, 800, &px, 200_000).unwrap();
        let r = analyse(&p, &Thresholds::default());
        assert!(!r.ok, "a solid fill must never pass: {r:?}");
        assert!(r.reason.contains("solid fill"), "{}", r.reason);
    }

    #[test]
    fn entropy_of_a_solid_fill_is_the_three_channel_floor_not_zero() {
        // The number that fooled v1. Entropy here is computed over three
        // separate channel histograms, so a single-colour image still puts one
        // bucket in each and scores log2(3) = 1.585. v1's threshold was 0.5, so
        // its entropy gate could never fail on a solid fill.
        let px = solid([16, 20, 28], 100, 100);
        let e = entropy(&px);
        assert!(
            (e - 3.0f64.log2()).abs() < 1e-9,
            "expected the log2(3) floor, got {e}"
        );
        assert!(
            e < Thresholds::default().min_entropy,
            "the threshold {0} must sit above the {1:.3} floor or the check is dead code",
            Thresholds::default().min_entropy,
            3.0f64.log2()
        );
    }

    #[test]
    fn a_window_passes() {
        let px = window(1280, 800);
        let p = Pixels::new("window.png", 1280, 800, &px, 120_000).unwrap();
        let r = analyse(&p, &Thresholds::default());
        assert!(r.ok, "a rendered window must pass: {r:?}");
    }

    #[test]
    fn a_tiny_file_is_rejected_even_when_it_looks_drawn() {
        let px = window(1280, 800);
        let p = Pixels::new("truncated.png", 1280, 800, &px, 12).unwrap();
        let r = analyse(&p, &Thresholds::default());
        assert!(!r.ok);
        assert!(r.reason.contains("truncated"), "{}", r.reason);
    }

    #[test]
    fn a_small_frame_is_rejected() {
        let px = window(320, 240);
        let p = Pixels::new("small.png", 320, 240, &px, 120_000).unwrap();
        let r = analyse(&p, &Thresholds::default());
        assert!(!r.ok);
        assert!(r.reason.contains("smaller than"), "{}", r.reason);
    }

    /// A genuinely smooth gradient: no wraparound, so no artificial edges.
    /// This is what a plymouth splash or a wallpaper looks like — many colours,
    /// high entropy, no interface.
    fn gradient(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                let g = ((x * 200) / w.max(1) + (y * 50) / h.max(1)) as u8;
                v.extend_from_slice(&[g / 2, g / 2, g]);
            }
        }
        v
    }

    #[test]
    fn a_smooth_gradient_with_no_text_is_rejected() {
        // The case that killed the previous "non-modal pixels" gate: this frame
        // has 200 colours and 8.5 bits of entropy, so every other check passes,
        // and it still shows nothing but a background.
        let (w, h) = (1280u32, 800u32);
        let v = gradient(w, h);
        let p = Pixels::new("gradient.png", w, h, &v, 90_000).unwrap();
        let r = analyse(&p, &Thresholds::default());
        assert!(
            r.unique_colors > Thresholds::default().min_unique_colors,
            "precondition: the gradient must have many colours, got {}",
            r.unique_colors
        );
        assert!(r.entropy > Thresholds::default().min_entropy);
        assert!(
            !r.ok,
            "a gradient with no interface must be rejected: {r:?}"
        );
        assert!(r.reason.contains("nothing was drawn"), "{}", r.reason);
    }

    #[test]
    fn edge_density_separates_a_window_from_a_gradient() {
        let px = window(400, 300);
        let edges = edge_density(&px, 400, 300, EDGE_STEP);
        let grad = gradient(400, 300);
        let gedges = edge_density(&grad, 400, 300, EDGE_STEP);
        assert!(
            edges > gedges * 20.0,
            "window edges {edges} should dwarf gradient edges {gedges}"
        );
    }

    #[test]
    fn a_solid_fill_has_no_edges() {
        let px = solid([30, 40, 60], 400, 300);
        assert_eq!(edge_density(&px, 400, 300, EDGE_STEP), 0.0);
    }

    #[test]
    fn entropy_and_ratio_are_computed_over_the_whole_frame() {
        let px = window(100, 100);
        assert!(entropy(&px) > 1.0);
        let t = text_ratio(&px);
        assert!(t > 0.0 && t < 1.0, "text ratio {t}");
    }

    #[test]
    fn unique_colours_respects_its_cap() {
        let (w, h) = (200u32, 200u32);
        let mut v = Vec::with_capacity((w * h * 3) as usize);
        for i in 0..(w * h) {
            v.extend_from_slice(&[(i % 256) as u8, (i / 256 % 256) as u8, 0]);
        }
        assert_eq!(unique_colors(&v, 1024), 1025, "cap is reported as cap+1");
    }

    #[test]
    fn a_short_buffer_is_rejected_rather_than_read_out_of_bounds() {
        assert!(Pixels::new("short.png", 100, 100, &[0u8; 10], 1000).is_none());
    }
}
