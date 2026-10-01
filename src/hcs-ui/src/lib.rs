//! HCS Linux â€” Neural Glass GUI foundation (plan: docs/GUI_BUILD_PLAN.md Â§2 P0).
//!
//! Slint is used under the **Slint Royalty-free License 2.0**
//! (`LicenseRef-Slint-Royalty-free-2.0`), which is the copyleft-free option of
//! Slint's triple license and therefore keeps HCS Linux under Apache-2.0.
//! The Royalty-free license requires disclosing the use of Slint; every app
//! built on this crate renders `disclosure()` in its About dialog and the
//! system Settings app lists it in the third-party section.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::rc::Rc;

slint::include_modules!();

pub mod harness;
pub use harness::{
    install as install_headless, last_window, render_to_png, HeadlessPlatform, RenderError,
};

pub mod image_studio;
pub use image_studio::{render_request_from_ui, ImageStudioState};

pub mod docs;
pub mod settings;

/// Raw 8-bit colour values for a theme preset. Kept as plain data so it can
/// cross crate boundaries; the generated `Palette` struct is crate-local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteValues {
    pub theme_name: &'static str,
    pub surface: (u8, u8, u8),
    pub surface_alt: (u8, u8, u8),
    pub surface_raised: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub accent_alt: (u8, u8, u8),
    pub text_primary: (u8, u8, u8),
    pub text_muted: (u8, u8, u8),
    pub border_soft: (u8, u8, u8),
    pub success: (u8, u8, u8),
    pub warn: (u8, u8, u8),
    pub danger: (u8, u8, u8),
}

pub fn rgb((r, g, b): (u8, u8, u8)) -> slint::Color {
    slint::Color::from_rgb_u8(r, g, b)
}

/// Colour values for a preset (mirrors the .slint `Presets` global).
pub fn palette_values(preset: ThemePreset) -> PaletteValues {
    match preset {
        ThemePreset::Obsidian => PaletteValues {
            theme_name: "obsidian",
            surface: (0x0d, 0x11, 0x17),
            surface_alt: (0x16, 0x1b, 0x22),
            surface_raised: (0x1e, 0x29, 0x3b),
            accent: (0x38, 0xbd, 0xf8),
            accent_alt: (0x81, 0x8c, 0xf8),
            text_primary: (0xe2, 0xe8, 0xf0),
            text_muted: (0x84, 0x94, 0xa8),
            border_soft: (0x1f, 0x29, 0x37),
            success: (0x34, 0xd3, 0x99),
            warn: (0xfb, 0xbf, 0x24),
            danger: (0xf8, 0x71, 0x71),
        },
        ThemePreset::Titanium => PaletteValues {
            theme_name: "titanium",
            surface: (0xe2, 0xe8, 0xf0),
            surface_alt: (0xcb, 0xd5, 0xe1),
            surface_raised: (0xf1, 0xf5, 0xf9),
            accent: (0x02, 0x84, 0xc7),
            accent_alt: (0x4f, 0x46, 0xe5),
            text_primary: (0x0f, 0x17, 0x2a),
            text_muted: (0x47, 0x55, 0x69),
            border_soft: (0x94, 0xa3, 0xb8),
            success: (0x05, 0x96, 0x69),
            warn: (0xb4, 0x53, 0x09),
            danger: (0xb9, 0x1c, 0x1c),
        },
        ThemePreset::Stealth => PaletteValues {
            theme_name: "stealth",
            surface: (0x0f, 0x0f, 0x16),
            surface_alt: (0x17, 0x17, 0x1f),
            surface_raised: (0x23, 0x23, 0x2e),
            accent: (0xc0, 0x84, 0xfc),
            accent_alt: (0x81, 0x8c, 0xf8),
            text_primary: (0xe5, 0xe7, 0xeb),
            text_muted: (0x6b, 0x72, 0x80),
            border_soft: (0x2a, 0x2a, 0x35),
            success: (0x34, 0xd3, 0x99),
            warn: (0xfb, 0xbf, 0x24),
            danger: (0xf8, 0x71, 0x71),
        },
        // WCAG AAA text pairs. Gate 10 asserts these ratios; if a value here is
        // ever "prettified" the gate must fail.
        ThemePreset::HighContrast => PaletteValues {
            theme_name: "high_contrast",
            surface: (0x00, 0x00, 0x00),
            surface_alt: (0x0a, 0x0a, 0x0a),
            surface_raised: (0x14, 0x14, 0x14),
            accent: (0x00, 0xe5, 0xff),
            accent_alt: (0xff, 0xff, 0x00),
            text_primary: (0xff, 0xff, 0xff),
            text_muted: (0xe0, 0xe0, 0xe0),
            border_soft: (0xff, 0xff, 0xff),
            success: (0x00, 0xff, 0x88),
            warn: (0xff, 0xa5, 0x00),
            danger: (0xff, 0x55, 0x55),
        },
    }
}

/// Build the crate-local generated `Palette` from a [`PaletteValues`].
///
/// The generated `Palette` type is emitted into every crate that imports the
/// kit by file path, so the struct literal must be written in the calling
/// crate (where `Palette` is in scope).
#[macro_export]
macro_rules! hcs_palette {
    ($v:expr) => {
        Palette {
            theme_name: $v.theme_name.into(),
            surface: $crate::rgb($v.surface),
            surface_alt: $crate::rgb($v.surface_alt),
            surface_raised: $crate::rgb($v.surface_raised),
            accent: $crate::rgb($v.accent),
            accent_alt: $crate::rgb($v.accent_alt),
            text_primary: $crate::rgb($v.text_primary),
            text_muted: $crate::rgb($v.text_muted),
            border_soft: $crate::rgb($v.border_soft),
            success: $crate::rgb($v.success),
            warn: $crate::rgb($v.warn),
            danger: $crate::rgb($v.danger),
        }
    };
}

/// Apply a theme preset to a window: sets the palette *and* the two toggles.
///
/// This must be called explicitly. A Slint `Palette` that is never assigned is
/// default-constructed with transparent colours, which renders an all-black
/// window — exactly the failure the blank-frame check in
/// `scripts/verify_gui.py` exists to catch.
///
/// Usage (from a module where the generated `Palette` is in scope):
/// ```ignore
/// hcs_ui::apply_hcs_theme!(win, ThemePreset::Obsidian);
/// ```
#[macro_export]
macro_rules! apply_hcs_theme {
    ($win:expr, $preset:expr) => {{
        let (titanium, stealth, high_contrast) = $crate::preset_flags($preset);
        $win.set_theme_titanium(titanium);
        $win.set_theme_stealth(stealth);
        $win.set_theme_high_contrast(high_contrast);
        $win.set_palette($crate::hcs_palette!($crate::palette_values($preset)));
        $win.set_theme_name($preset.as_str().into());
    }};
}

// `include_modules!()` above brings the generated types (`AppFrame`, `Palette`,
// `Presets`, `WidgetGallery`, ...) into the crate root, so app crates and the
// render harness can simply `use hcs_ui::AppFrame;`.

/// Budget ceilings from the master plan RAM matrix (docs/V1_STABLE_RELEASE_MASTER_PLAN.md Â§5).
pub const IDLE_BUDGET_MB: f32 = 6144.0;
pub const PEAK_BUDGET_MB: f32 = 8192.0;
/// Per-application ceiling for GUI apps.
pub const APP_RSS_BUDGET_MB: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreset {
    /// Obsidian Neural Gradient — default dark theme.
    Obsidian,
    /// Frosted Titanium — light architectural theme.
    Titanium,
    /// Cybernetic Stealth — Security Lab theme.
    Stealth,
    /// High Contrast — accessibility preset, checked by the Gate 10 audit.
    ///
    /// This is not a brand theme. It exists because Apple needed four Liquid
    /// Glass revisions before translucent controls were legible again; we ship
    /// the corrected version as a first-class preset instead of rediscovering
    /// the bug.
    HighContrast,
}

impl ThemePreset {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemePreset::Obsidian => "obsidian",
            ThemePreset::Titanium => "titanium",
            ThemePreset::Stealth => "stealth",
            ThemePreset::HighContrast => "high_contrast",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "obsidian" => Some(ThemePreset::Obsidian),
            "titanium" => Some(ThemePreset::Titanium),
            "stealth" => Some(ThemePreset::Stealth),
            "high_contrast" | "high-contrast" | "contrast" => Some(ThemePreset::HighContrast),
            _ => None,
        }
    }

    /// Every preset, in menu order. Used by `hcs theme list` and the settings
    /// app so the list cannot drift from the enum.
    pub fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::Obsidian,
            ThemePreset::Titanium,
            ThemePreset::Stealth,
            ThemePreset::HighContrast,
        ]
    }

    /// The wallpaper that belongs to this preset, matching `colors.toml`.
    pub fn wallpaper(self) -> &'static str {
        match self {
            ThemePreset::Titanium => "frosted_titanium",
            ThemePreset::Stealth => "cybernetic_stealth",
            // High Contrast keeps the default wallpaper: legibility comes from
            // the palette, not from an image nobody can see through.
            ThemePreset::Obsidian | ThemePreset::HighContrast => "neural_glass_dark",
        }
    }
}

/// Mandatory Slint attribution text (Royalty-free License 2.0 disclosure).
pub fn disclosure() -> &'static str {
    "This application is built with Slint (https://slint.dev), \
     used under the Slint Royalty-free License 2.0."
}

/// Persist the active theme so the shell (QML) and the Slint apps agree.
pub fn theme_config_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        std::path::PathBuf::from(dir).join("hcs/theme.json")
    } else {
        std::path::PathBuf::from("/etc/hcs/theme.json")
    }
}

/// Booleans that select the active Neural Glass preset. The palette itself is
/// resolved inside the window (`theme-titanium` / `theme-stealth` /
/// `theme-high-contrast`), so plain `bool` bindings are all that is needed —
/// those are always code-generated, which keeps the theme system testable from
/// `cargo test`.
pub fn preset_flags(preset: ThemePreset) -> (bool, bool, bool) {
    match preset {
        ThemePreset::Obsidian => (false, false, false),
        ThemePreset::Titanium => (true, false, false),
        ThemePreset::Stealth => (false, true, false),
        ThemePreset::HighContrast => (false, false, true),
    }
}

/// Implemented for every generated window type so the theme can be applied
/// without knowing the concrete type.
pub trait ThemeTarget {
    fn apply_theme_flags(&self, titanium: bool, stealth: bool, high_contrast: bool);
}

/// Implement [`ThemeTarget`] for a generated Slint window type.
/// Usage: `hcs_ui::theme_target!(hcs_ui::WidgetGallery);`
#[macro_export]
macro_rules! theme_target {
    ($t:ty) => {
        impl $crate::ThemeTarget for $t {
            fn apply_theme_flags(&self, titanium: bool, stealth: bool, high_contrast: bool) {
                self.set_theme_titanium(titanium);
                self.set_theme_stealth(stealth);
                self.set_theme_high_contrast(high_contrast);
            }
        }
    };
}

theme_target!(WidgetGallery);

/// Build the foundation gallery with a theme already applied.
///
/// The render tool cannot theme a window itself: the generated `Palette` type
/// is crate-local, so the palette must be assigned in the crate that owns the
/// component.
pub fn build_gallery_themed(theme: ThemePreset) -> Result<WidgetGallery, slint::PlatformError> {
    let w = WidgetGallery::new()?;
    apply_hcs_theme!(w, theme);
    Ok(w)
}

/// Read the persisted theme preset (defaults to Obsidian).
pub fn load_theme() -> ThemePreset {
    let p = theme_config_path();
    if let Ok(txt) = std::fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
            if let Some(name) = v.get("theme").and_then(|x| x.as_str()) {
                if let Some(preset) = ThemePreset::from_str_opt(name) {
                    return preset;
                }
            }
        }
    }
    ThemePreset::Obsidian
}
pub fn save_theme(preset: ThemePreset) -> std::io::Result<()> {
    let p = theme_config_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({ "theme": preset.as_str() });
    std::fs::write(
        &p,
        serde_json::to_string_pretty(&payload).unwrap_or_default(),
    )
}

/// RAM budget helper shared by monitor + image studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamBudget {
    pub used_mb: u64,
    pub idle_budget_mb: u64,
    pub peak_budget_mb: u64,
}

impl RamBudget {
    pub fn new(used_mb: u64) -> Self {
        Self {
            used_mb,
            idle_budget_mb: IDLE_BUDGET_MB as u64,
            peak_budget_mb: PEAK_BUDGET_MB as u64,
        }
    }

    pub fn within_idle(&self) -> bool {
        self.used_mb <= self.idle_budget_mb
    }

    pub fn within_peak(&self) -> bool {
        self.used_mb <= self.peak_budget_mb
    }

    pub fn fraction_of_idle(&self) -> f32 {
        self.used_mb as f32 / self.idle_budget_mb as f32
    }
}

/// Steps clamp for the Image Studio (LCM supports 1-8 steps).
pub fn clamp_steps(steps: f32) -> i32 {
    steps.round().clamp(1.0, 8.0) as i32
}

/// CFG clamp for the Image Studio.
pub fn clamp_cfg(cfg: f32) -> f32 {
    cfg.clamp(1.0, 2.5)
}

/// Process RSS in MB, read from /proc (Linux). None on other platforms.
pub fn process_rss_mb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: u64 = rest.trim().trim_end_matches(" kB").trim().parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
}

/// Ensure a writable output directory exists for screenshots.
pub fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    Ok(())
}

pub type SharedTheme = Rc<()>;
