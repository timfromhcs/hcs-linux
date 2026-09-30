//! HCS Screenshot — capture, OCR extract, colour pick, screen record.
//!
//! Gap closed from the v2 audit (B-11). Patterns taken:
//!   - Windows 11 Snipping Tool: "perfect screenshot" framing, text extractor
//!     (OCR), colour picker, and screen recording with microphone, all in one
//!     tool.
//!   - iOS 26 / macOS 27 Visual Intelligence: select something on screen, ask
//!     what it means. `hcs-shot --extract` is the front end for that, and
//!     `hcs-shot --color` is the colour picker's output format.
//!
//! The capture backends differ per environment (Wayland portal, X11, or the
//! QA agent), so the *decision logic* lives here in a pure module and the
//! backend is injected. That keeps the geometry, timing and filename rules
//! testable without a compositor.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// What to capture.
///
/// Not `Copy`: the delayed variant nests a boxed target, and boxing keeps the
/// enum small enough that a slice of targets stays cheap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// Whole screen.
    Screen,
    /// A user- or agent-selected rectangle, in physical pixels.
    Region { x: i32, y: i32, w: u32, h: u32 },
    /// A single window, by title substring.
    Window { title: String },
    /// The focused window.
    ActiveWindow,
    /// A delay in seconds before the capture fires, for "grab that menu".
    Delayed { secs: u64, target: Box<Target> },
}

/// Output encoding. PNG is the default because screenshots of text must not be
/// lossily compressed — an OCR pass on a JPEG artifact reads garbage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Png,
    Jpeg,
    Webp,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
            Format::Webp => "webp",
        }
    }

    /// Lossy formats refuse OCR. Saying so up front beats returning a
    /// confident, wrong transcription.
    pub fn supports_ocr(self) -> bool {
        matches!(self, Format::Png | Format::Webp)
    }
}

/// A complete capture request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub target: Target,
    pub format: Format,
    pub output: Option<PathBuf>,
    /// Freeform tag used in the filename, e.g. "omnibar" or "snap-layouts".
    pub label: Option<String>,
}

impl Default for CaptureRequest {
    fn default() -> Self {
        Self {
            target: Target::Screen,
            format: Format::Png,
            output: None,
            label: None,
        }
    }
}

/// A capture that has been taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capture {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub format: Format,
}

/// Errors that are user-actionable, so the CLI can print the next step rather
/// than a bare failure (GEMINI §152 Failure UX).
#[derive(Debug, thiserror::Error)]
pub enum ShotError {
    #[error("no capture backend available — a graphical session with a portal is required")]
    NoBackend,
    #[error("region is empty: width and height must both be greater than zero")]
    EmptyRegion,
    #[error("region {0}x{1} exceeds the {2}x{3} screen")]
    RegionOffscreen(u32, u32, u32, u32),
    #[error("no window matched title containing '{0}'")]
    NoWindowMatch(String),
    #[error("OCR is not available for {0:?} — capture as PNG instead")]
    OcrUnsupported(Format),
    #[error("no OCR backend installed (tesseract-ocr)")]
    NoOcr,
    #[error("failed to write {0}: {1}")]
    Write(PathBuf, std::io::Error),
}

/// A capture backend. Implemented for the Wayland portal, for X11, and by the
/// in-guest QA agent.
pub trait Backend {
    fn name(&self) -> &'static str;
    fn capture(&mut self, req: &CaptureRequest) -> Result<Capture, ShotError>;
}

/// Reject an OCR request whose capture format cannot carry text faithfully.
///
/// A lossy screenshot OCR'd into plausible-but-wrong words is worse than no
/// answer at all, because the caller cannot tell the difference. This gate makes
/// that impossible instead of merely unlikely.
pub fn ocr_gate(req: &CaptureRequest) -> Result<(), ShotError> {
    if req.format.supports_ocr() {
        Ok(())
    } else {
        Err(ShotError::OcrUnsupported(req.format))
    }
}

/// Validate a request against the screen geometry before any backend runs.
///
/// Catching an off-screen or zero-size region here means the user gets an
/// immediate, precise error instead of a backend-specific failure — or worse, a
/// silent black PNG that only the entropy gate catches later.
pub fn validate(req: &CaptureRequest, screen_w: u32, screen_h: u32) -> Result<(), ShotError> {
    match &req.target {
        Target::Region { x, y, w, h } => {
            if *w == 0 || *h == 0 {
                return Err(ShotError::EmptyRegion);
            }
            let right = *x + *w as i32;
            let bottom = *y + *h as i32;
            if right > screen_w as i32 || bottom > screen_h as i32 {
                return Err(ShotError::RegionOffscreen(*w, *h, screen_w, screen_h));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Where a capture is written when no explicit path was given.
///
/// Mirrors how a screenshot tool should behave: dated, sorted, and labelled by
/// the surface that produced it, so a QA reviewer can find `19_omnibar.png`
/// without opening a single image.
pub fn default_output(dir: &Path, label: Option<&str>, format: Format, stamp: &str) -> PathBuf {
    let name = match label {
        Some(l) => format!("{stamp}_{l}.{}", format.extension()),
        None => format!("{stamp}.{}", format.extension()),
    };
    dir.join(name)
}

/// Human-readable region description for the status line.
pub fn describe(target: &Target) -> String {
    match target {
        Target::Screen => "entire screen".to_string(),
        Target::ActiveWindow => "focused window".to_string(),
        Target::Window { title } => format!("window matching \"{title}\""),
        Target::Region { x, y, w, h } => format!("region {x},{y} {w}×{h}"),
        Target::Delayed { secs, target } => format!("{secs}s delay then {}", describe(target)),
    }
}

/// A colour read off the screen, in the format a designer can paste into CSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// WCAG relative luminance. Gate 10 uses this to assert that every theme
    /// text pair clears the required ratio.
    pub fn relative_luminance(self) -> f32 {
        fn channel(c: u8) -> f32 {
            let c = c as f32 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }
}

impl Rgb {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// WCAG contrast ratio between two colours, 1.0 (identical) to 21.0.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f32 {
    let la = a.relative_luminance();
    let lb = b.relative_luminance();
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// A text-extraction result. Word boxes are kept so the caller can highlight
/// what it read, which is what makes "explain this" possible later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    pub words: Vec<OcrWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrWord {
    pub text: String,
    /// Confidence 0.0–1.0. Words below a threshold are reported but flagged,
    /// never silently dropped.
    pub confidence: f32,
}

impl OcrResult {
    /// Mean confidence across recognised words. Zero words means zero
    /// confidence, not full confidence — an empty result must never look
    /// authoritative.
    pub fn mean_confidence(&self) -> f32 {
        if self.words.is_empty() {
            return 0.0;
        }
        self.words.iter().map(|w| w.confidence).sum::<f32>() / self.words.len() as f32
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> CaptureRequest {
        CaptureRequest::default()
    }

    #[test]
    fn zero_region_is_rejected() {
        let req = CaptureRequest {
            target: Target::Region {
                x: 0,
                y: 0,
                w: 0,
                h: 100,
            },
            ..screen()
        };
        assert!(matches!(
            validate(&req, 1920, 1080),
            Err(ShotError::EmptyRegion)
        ));
    }

    #[test]
    fn offscreen_region_is_rejected() {
        let req = CaptureRequest {
            target: Target::Region {
                x: 1800,
                y: 0,
                w: 400,
                h: 200,
            },
            ..screen()
        };
        assert!(matches!(
            validate(&req, 1920, 1080),
            Err(ShotError::RegionOffscreen(400, 200, 1920, 1080))
        ));
    }

    #[test]
    fn region_ending_exactly_on_the_edge_is_accepted() {
        let req = CaptureRequest {
            target: Target::Region {
                x: 1520,
                y: 880,
                w: 400,
                h: 200,
            },
            ..screen()
        };
        assert!(validate(&req, 1920, 1080).is_ok());
    }

    #[test]
    fn screen_target_never_fails_validation() {
        assert!(validate(&screen(), 0, 0).is_ok());
    }

    #[test]
    fn default_output_includes_label_and_stamp() {
        let p = default_output(Path::new("/tmp"), Some("omnibar"), Format::Png, "19");
        assert_eq!(p, PathBuf::from("/tmp/19_omnibar.png"));
    }

    #[test]
    fn default_output_without_label_uses_plain_stamp() {
        let p = default_output(Path::new("/tmp"), None, Format::Jpeg, "20");
        assert_eq!(p, PathBuf::from("/tmp/20.jpg"));
    }

    #[test]
    fn describe_renders_each_target() {
        assert_eq!(describe(&Target::Screen), "entire screen");
        assert_eq!(describe(&Target::ActiveWindow), "focused window");
        assert_eq!(
            describe(&Target::Window {
                title: "AI Chat".into()
            }),
            "window matching \"AI Chat\""
        );
        assert_eq!(
            describe(&Target::Region {
                x: 1,
                y: 2,
                w: 3,
                h: 4
            }),
            "region 1,2 3×4"
        );
        assert_eq!(
            describe(&Target::Delayed {
                secs: 2,
                target: Box::new(Target::Screen)
            }),
            "2s delay then entire screen"
        );
    }

    #[test]
    fn png_supports_ocr_jpeg_does_not() {
        assert!(Format::Png.supports_ocr());
        assert!(!Format::Jpeg.supports_ocr());
        assert!(matches!(
            ocr_gate(&CaptureRequest {
                format: Format::Jpeg,
                ..screen()
            }),
            Err(ShotError::OcrUnsupported(Format::Jpeg))
        ));
    }

    #[test]
    fn ocr_gate_passes_for_png() {
        assert!(ocr_gate(&CaptureRequest {
            format: Format::Png,
            ..screen()
        })
        .is_ok());
    }

    #[test]
    fn hex_is_lowercase_six_digits() {
        assert_eq!(Rgb::from_rgb(0x38, 0xbd, 0xf8).hex(), "#38bdf8");
        assert_eq!(Rgb::from_rgb(0, 0, 0).hex(), "#000000");
    }

    #[test]
    fn contrast_of_identical_colours_is_one() {
        let c = Rgb::from_rgb(0x11, 0x22, 0x33);
        assert!((contrast_ratio(c, c) - 1.0).abs() < 0.001);
    }

    #[test]
    fn black_on_white_is_maximum_contrast() {
        let ratio = contrast_ratio(Rgb::from_rgb(0, 0, 0), Rgb::from_rgb(255, 255, 255));
        assert!((ratio - 21.0).abs() < 0.01, "got {ratio}");
    }

    #[test]
    fn contrast_is_symmetric() {
        let a = Rgb::from_rgb(0x0d, 0x11, 0x17);
        let b = Rgb::from_rgb(0xe6, 0xed, 0xf3);
        assert!((contrast_ratio(a, b) - contrast_ratio(b, a)).abs() < 1e-6);
    }

    #[test]
    fn obsidian_body_text_clears_wcag_aa() {
        let bg = Rgb::from_rgb(0x0d, 0x11, 0x17);
        let fg = Rgb::from_rgb(0xe6, 0xed, 0xf3);
        assert!(
            contrast_ratio(fg, bg) >= 4.5,
            "obsidian body text must clear AA, got {:.2}",
            contrast_ratio(fg, bg)
        );
    }

    #[test]
    fn obsidian_muted_text_clears_wcag_aa() {
        let bg = Rgb::from_rgb(0x0d, 0x11, 0x17);
        let fg = Rgb::from_rgb(0x8b, 0x94, 0x9e);
        let ratio = contrast_ratio(fg, bg);
        assert!(
            ratio >= 4.5,
            "muted text must still clear AA, got {ratio:.2}"
        );
    }

    #[test]
    fn high_contrast_preset_clears_wcag_aaa() {
        let bg = Rgb::from_rgb(0, 0, 0);
        for fg in [(255u8, 255u8, 255u8), (224, 224, 224)] {
            let ratio = contrast_ratio(Rgb::from_rgb(fg.0, fg.1, fg.2), bg);
            assert!(ratio >= 7.0, "high contrast must clear AAA, got {ratio:.2}");
        }
    }

    #[test]
    fn ocr_result_reports_empty_as_low_confidence() {
        let r = OcrResult {
            text: String::new(),
            words: vec![],
        };
        assert!(r.is_empty());
        assert_eq!(r.mean_confidence(), 0.0);
    }

    #[test]
    fn ocr_mean_confidence_averages_words() {
        let r = OcrResult {
            text: "hello world".into(),
            words: vec![
                OcrWord {
                    text: "hello".into(),
                    confidence: 0.8,
                },
                OcrWord {
                    text: "world".into(),
                    confidence: 1.0,
                },
            ],
        };
        assert!((r.mean_confidence() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn window_title_with_no_match_is_reported() {
        let err = ShotError::NoWindowMatch("AI Chat".into());
        assert!(err.to_string().contains("AI Chat"));
    }

    #[test]
    fn missing_backend_error_names_the_requirement() {
        assert!(ShotError::NoBackend
            .to_string()
            .contains("graphical session"));
    }
}
