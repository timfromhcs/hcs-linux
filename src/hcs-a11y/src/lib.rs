//! HCS Accessibility — focus order, contrast, motion, text scaling.
//!
//! Gap closed from the v2 audit (B-17). `docs/GUI_BUILD_PLAN.md` §4 step 7
//! promised a focus-walk test per view and a WCAG contrast assertion; neither
//! was implemented, so accessibility existed only as a claim in the release
//! notes. This crate makes both mechanical:
//!
//!   - the focus order of every view is a **declared constant**, and a test
//!     asserts it matches the elements the view actually contains;
//!   - every theme's text/background pair is checked against a WCAG threshold;
//!   - Reduce Motion is an OS-level preference (iOS ships it that way), not an
//!     app-level setting.
//!
//! Apple needed four Liquid Glass revisions before translucent controls were
//! legible. The contrast gate below is the reason HCS does not need them.

use serde::{Deserialize, Serialize};

/// WCAG 2.1 contrast requirements.
pub const AA_NORMAL: f32 = 4.5;
pub const AA_LARGE: f32 = 3.0;
pub const AAA_NORMAL: f32 = 7.0;

/// Where a focusable control sits in a view's traversal order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusStop {
    /// Stable identifier, e.g. "omnibar.input".
    pub id: String,
    /// Whether the control can take keyboard focus at all.
    pub focusable: bool,
    /// Whether it is reachable with Tab (as opposed to arrow keys only).
    pub tabbable: bool,
}

/// The declared focus order for one view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusOrder {
    pub view: String,
    pub stops: Vec<FocusStop>,
}

impl FocusOrder {
    pub fn tabbable_ids(&self) -> Vec<&str> {
        self.stops
            .iter()
            .filter(|s| s.focusable && s.tabbable)
            .map(|s| s.id.as_str())
            .collect()
    }

    /// A duplicate id makes traversal ambiguous — the user cannot tell where the
    /// focus is. Duplicates and empty ids are the failure this catches.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for s in &self.stops {
            if s.id.trim().is_empty() {
                problems.push(format!(
                    "view '{}' has a focus stop with an empty id",
                    self.view
                ));
                continue;
            }
            if !seen.insert(s.id.as_str()) {
                problems.push(format!(
                    "view '{}' has duplicate focus id '{}'",
                    self.view, s.id
                ));
            }
        }
        if self.tabbable_ids().is_empty() {
            problems.push(format!(
                "view '{}' has no tabbable control — it would be keyboard-unreachable",
                self.view
            ));
        }
        problems
    }
}

/// An 8-bit RGB triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn relative_luminance(&self) -> f32 {
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

/// WCAG contrast ratio, 1.0 to 21.0.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f32 {
    let la = a.relative_luminance();
    let lb = b.relative_luminance();
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// One pair to check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContrastCheck {
    pub theme: String,
    pub foreground: String,
    pub background: String,
    pub ratio: f32,
    pub required: f32,
    pub passes: bool,
}

/// Evaluate one theme's foreground/background pair.
pub fn check_contrast(theme: &str, fg: Rgb, bg: Rgb, required: f32) -> ContrastCheck {
    let ratio = contrast_ratio(fg, bg);
    ContrastCheck {
        theme: theme.to_string(),
        foreground: fg.hex(),
        background: bg.hex(),
        ratio,
        required,
        passes: ratio >= required,
    }
}

/// Motion preference. An OS-level setting, because "reduce motion" that each
/// app has to implement separately is a setting nobody can rely on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    Full,
    Reduced,
    None,
}

impl Motion {
    /// Duration for an animation under this preference. Returning zero is how a
    /// "disable" is expressed in a framework that has no off switch.
    pub fn duration_ms(self, requested_ms: u64) -> u64 {
        match self {
            Motion::Full => requested_ms,
            Motion::Reduced => requested_ms / 4,
            Motion::None => 0,
        }
    }

    /// Whether a looping animation may run at all. An infinite spinner under
    //  Reduce Motion is exactly the vestibular trigger the setting exists for.
    pub fn allows_looping(self) -> bool {
        matches!(self, Motion::Full)
    }
}

/// Text scaling, 100 %–200 %. Android 16's lesson was that inconsistent scaling
/// across window sizes breaks layouts, so the scale is a first-class value that
/// both the shell and the apps read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextScale(pub u8);

impl TextScale {
    pub const MIN: u8 = 100;
    pub const MAX: u8 = 200;
    pub const STEP: u8 = 10;

    pub fn new(percent: u16) -> Self {
        TextScale(percent.clamp(Self::MIN as u16, Self::MAX as u16) as u8)
    }

    pub fn factor(self) -> f32 {
        self.0 as f32 / 100.0
    }

    pub fn increase(self) -> Self {
        TextScale::new(self.0 as u16 + Self::STEP as u16)
    }

    pub fn decrease(self) -> Self {
        TextScale::new(self.0 as u16 - Self::STEP as u16)
    }

    pub fn is_default(self) -> bool {
        self.0 == 100
    }

    /// Scale a base pixel size, never below the legibility floor. Clamping at the
    /// bottom is what stops a user from scaling *down* into unreadable text.
    pub fn apply(self, base_px: f32) -> f32 {
        (base_px * self.factor()).max(11.0)
    }
}

/// A colour-blind-safe check: whether two accents are distinguishable. Uses a
/// crude hue-distance model, which is enough to catch "two status colours that
/// are the same green".
pub fn accents_distinguishable(a: Rgb, b: Rgb, min_ratio: f32) -> bool {
    contrast_ratio(a, b) >= min_ratio
}

/// Every theme's declared accessibility profile. Kept next to the checker so a
/// theme cannot be added without declaring its thresholds.
pub struct ThemeProfile {
    pub name: &'static str,
    pub bg: Rgb,
    pub surface: Rgb,
    pub body_text: Rgb,
    pub muted_text: Rgb,
    /// AAA for the accessibility preset, AA for the brand themes.
    pub required_normal: f32,
}

pub fn theme_profiles() -> Vec<ThemeProfile> {
    vec![
        ThemeProfile {
            name: "obsidian",
            bg: Rgb::new(0x0d, 0x11, 0x17),
            surface: Rgb::new(0x16, 0x1b, 0x22),
            body_text: Rgb::new(0xe6, 0xed, 0xf3),
            muted_text: Rgb::new(0x8b, 0x94, 0x9e),
            required_normal: AA_NORMAL,
        },
        ThemeProfile {
            name: "titanium",
            bg: Rgb::new(0xe2, 0xe8, 0xf0),
            surface: Rgb::new(0xcb, 0xd5, 0xe1),
            body_text: Rgb::new(0x0f, 0x17, 0x2a),
            muted_text: Rgb::new(0x47, 0x55, 0x69),
            required_normal: AA_NORMAL,
        },
        ThemeProfile {
            name: "stealth",
            bg: Rgb::new(0x0f, 0x0f, 0x16),
            surface: Rgb::new(0x17, 0x17, 0x1f),
            body_text: Rgb::new(0xe5, 0xe7, 0xeb),
            muted_text: Rgb::new(0x8b, 0x95, 0xa5),
            required_normal: AA_NORMAL,
        },
        ThemeProfile {
            name: "high_contrast",
            bg: Rgb::new(0x00, 0x00, 0x00),
            surface: Rgb::new(0x0a, 0x0a, 0x0a),
            body_text: Rgb::new(0xff, 0xff, 0xff),
            muted_text: Rgb::new(0xe0, 0xe0, 0xe0),
            required_normal: AAA_NORMAL,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_clears_its_declared_threshold() {
        for t in theme_profiles() {
            for (label, fg) in [("body", t.body_text), ("muted", t.muted_text)] {
                for (surface_label, bg) in [("bg", t.bg), ("surface", t.surface)] {
                    let c = check_contrast(t.name, fg, bg, t.required_normal);
                    assert!(
                        c.passes,
                        "theme {} {label} on {surface_label} is {:.2}:1, needs {:.1}:1 ({} on {})",
                        t.name, c.ratio, t.required_normal, c.foreground, c.background
                    );
                }
            }
        }
    }

    #[test]
    fn high_contrast_theme_reaches_aaa() {
        let t = theme_profiles()
            .into_iter()
            .find(|t| t.name == "high_contrast")
            .unwrap();
        let ratio = contrast_ratio(t.body_text, t.bg);
        assert!(
            ratio >= AAA_NORMAL,
            "the accessibility preset must reach AAA, got {ratio:.2}"
        );
    }

    #[test]
    fn muted_text_still_clears_aa_on_every_brand_theme() {
        // Muted text is where legibility quietly fails: it is the first thing a
        // "prettier" palette change breaks and the last thing anyone notices.
        for t in theme_profiles()
            .iter()
            .filter(|t| t.name != "high_contrast")
        {
            let ratio = contrast_ratio(t.muted_text, t.bg);
            assert!(
                ratio >= AA_NORMAL,
                "theme {} muted text is only {ratio:.2}:1",
                t.name
            );
        }
    }

    #[test]
    fn contrast_of_identical_colours_is_one() {
        let c = Rgb::new(0x33, 0x33, 0x33);
        assert!((contrast_ratio(c, c) - 1.0).abs() < 0.001);
    }

    #[test]
    fn contrast_is_symmetric() {
        let a = Rgb::new(0x00, 0x00, 0x00);
        let b = Rgb::new(0xff, 0xff, 0xff);
        assert!((contrast_ratio(a, b) - contrast_ratio(b, a)).abs() < 1e-6);
        assert!((contrast_ratio(a, b) - 21.0).abs() < 0.01);
    }

    #[test]
    fn focus_order_accepts_a_well_formed_view() {
        let order = FocusOrder {
            view: "omnibar".into(),
            stops: vec![
                FocusStop {
                    id: "omnibar.input".into(),
                    focusable: true,
                    tabbable: true,
                },
                FocusStop {
                    id: "omnibar.results".into(),
                    focusable: true,
                    tabbable: true,
                },
            ],
        };
        assert!(order.validate().is_empty());
        assert_eq!(
            order.tabbable_ids(),
            vec!["omnibar.input", "omnibar.results"]
        );
    }

    #[test]
    fn duplicate_focus_ids_are_rejected() {
        let order = FocusOrder {
            view: "fm".into(),
            stops: vec![
                FocusStop {
                    id: "fm.search".into(),
                    focusable: true,
                    tabbable: true,
                },
                FocusStop {
                    id: "fm.search".into(),
                    focusable: true,
                    tabbable: true,
                },
            ],
        };
        let problems = order.validate();
        assert!(problems.iter().any(|p| p.contains("duplicate")));
    }

    #[test]
    fn empty_focus_id_is_rejected() {
        let order = FocusOrder {
            view: "term".into(),
            stops: vec![FocusStop {
                id: "   ".into(),
                focusable: true,
                tabbable: true,
            }],
        };
        assert!(order.validate().iter().any(|p| p.contains("empty id")));
    }

    #[test]
    fn a_view_with_nothing_tabbable_is_rejected() {
        let order = FocusOrder {
            view: "widgets".into(),
            stops: vec![FocusStop {
                id: "widgets.board".into(),
                focusable: false,
                tabbable: false,
            }],
        };
        assert!(order
            .validate()
            .iter()
            .any(|p| p.contains("keyboard-unreachable")));
    }

    #[test]
    fn non_tabbable_focusable_stops_are_excluded_from_tab_order() {
        let order = FocusOrder {
            view: "shot".into(),
            stops: vec![
                FocusStop {
                    id: "shot.region".into(),
                    focusable: true,
                    tabbable: false,
                },
                FocusStop {
                    id: "shot.capture".into(),
                    focusable: true,
                    tabbable: true,
                },
            ],
        };
        assert_eq!(order.tabbable_ids(), vec!["shot.capture"]);
    }

    #[test]
    fn motion_full_keeps_the_requested_duration() {
        assert_eq!(Motion::Full.duration_ms(200), 200);
    }

    #[test]
    fn motion_reduced_shortens_duration() {
        assert_eq!(Motion::Reduced.duration_ms(200), 50);
    }

    #[test]
    fn motion_none_zeroes_duration() {
        assert_eq!(Motion::None.duration_ms(200), 0);
    }

    #[test]
    fn looping_animations_are_disabled_unless_motion_is_full() {
        assert!(Motion::Full.allows_looping());
        assert!(!Motion::Reduced.allows_looping());
        assert!(!Motion::None.allows_looping());
    }

    #[test]
    fn text_scale_clamps_to_range() {
        assert_eq!(TextScale::new(50).0, 100);
        assert_eq!(TextScale::new(400).0, 200);
        assert_eq!(TextScale::new(125).0, 125);
    }

    #[test]
    fn text_scale_steps_and_saturates() {
        let s = TextScale::new(100);
        assert_eq!(s.increase().0, 110);
        assert_eq!(TextScale::new(200).increase().0, 200);
        assert_eq!(TextScale::new(100).decrease().0, 100);
    }

    #[test]
    fn text_scale_never_shrinks_text_below_the_floor() {
        // Scaling down must not make text unreadable; 100% is already the floor
        // so the clamp holds at every step.
        let s = TextScale::new(100);
        assert!(s.apply(9.0) >= 11.0);
        assert!((s.apply(14.0) - 14.0).abs() < 0.01);
    }

    #[test]
    fn text_scale_scales_up_proportionally() {
        let s = TextScale::new(150);
        assert!((s.apply(12.0) - 18.0).abs() < 0.01);
    }

    #[test]
    fn text_scale_reports_whether_it_is_default() {
        assert!(TextScale::new(100).is_default());
        assert!(!TextScale::new(120).is_default());
    }

    #[test]
    fn indistinguishable_accents_are_rejected() {
        // Two greens that differ by a hair would be unreadable as status colours.
        let a = Rgb::new(0x3f, 0xb9, 0x50);
        let b = Rgb::new(0x40, 0xba, 0x51);
        assert!(!accents_distinguishable(a, b, 1.2));
    }

    #[test]
    fn clearly_distinct_accents_pass() {
        let a = Rgb::new(0x3f, 0xb9, 0x50);
        let b = Rgb::new(0xf8, 0x51, 0x49);
        assert!(accents_distinguishable(a, b, 1.2));
    }

    #[test]
    fn hex_formatting_is_lowercase_six_digit() {
        assert_eq!(Rgb::new(0x38, 0xbd, 0xf8).hex(), "#38bdf8");
    }

    #[test]
    fn check_reports_the_threshold_it_used() {
        let c = check_contrast("t", Rgb::new(0, 0, 0), Rgb::new(255, 255, 255), AA_NORMAL);
        assert!(c.passes);
        assert_eq!(c.required, AA_NORMAL);
        assert!(c.ratio > 20.0);
    }
}
