//! hcs-ui unit tests. No display, no GPU: the Slint test API and the headless
//! harness work in-process. Plan §4 step 1.

use hcs_ui::{ThemePreset, WidgetGallery};

#[test]
fn theme_preset_roundtrip() {
    for p in [
        ThemePreset::Obsidian,
        ThemePreset::Titanium,
        ThemePreset::Stealth,
    ] {
        assert_eq!(ThemePreset::from_str_opt(p.as_str()), Some(p));
    }
    assert_eq!(ThemePreset::from_str_opt("nonsense"), None);
}

#[test]
fn preset_flags_are_mutually_exclusive() {
    assert_eq!(hcs_ui::preset_flags(ThemePreset::Obsidian), (false, false));
    assert_eq!(hcs_ui::preset_flags(ThemePreset::Titanium), (true, false));
    assert_eq!(hcs_ui::preset_flags(ThemePreset::Stealth), (false, true));
}

#[test]
fn theme_toggles_change_resolved_theme_name() {
    let g = WidgetGallery::new().expect("gallery");
    assert_eq!(g.get_theme_name().as_str(), "obsidian");
    g.set_theme_titanium(true);
    assert_eq!(g.get_theme_name().as_str(), "titanium");
    g.set_theme_titanium(false);
    g.set_theme_stealth(true);
    assert_eq!(g.get_theme_name().as_str(), "stealth");
}

#[test]
fn ram_budget_matrix() {
    let idle = hcs_ui::RamBudget::new(1580);
    assert!(idle.within_idle());
    assert!(idle.within_peak());
    assert!(idle.fraction_of_idle() > 0.2 && idle.fraction_of_idle() < 0.3);

    let peak = hcs_ui::RamBudget::new(8000);
    assert!(
        !peak.within_idle(),
        "8000MB must exceed the 6144MB idle cap"
    );
    assert!(
        peak.within_peak(),
        "8000MB must stay under the 8192MB peak cap"
    );

    let over = hcs_ui::RamBudget::new(9000);
    assert!(!over.within_peak());
}

#[test]
fn image_studio_clamps_follow_sd_lcm_limits() {
    // LCM: 1-8 steps, CFG 1.0-2.5 (master plan §4.2)
    assert_eq!(hcs_ui::clamp_steps(0.0), 1);
    assert_eq!(hcs_ui::clamp_steps(6.4), 6);
    assert_eq!(hcs_ui::clamp_steps(99.0), 8);
    assert_eq!(hcs_ui::clamp_cfg(0.1), 1.0);
    assert_eq!(hcs_ui::clamp_cfg(1.8), 1.8);
    assert_eq!(hcs_ui::clamp_cfg(9.0), 2.5);
}

#[test]
fn disclosure_text_is_present() {
    let d = hcs_ui::disclosure();
    assert!(d.contains("Slint"));
    assert!(
        d.contains("Royalty-free"),
        "license disclosure must name the license"
    );
}

#[test]
fn app_budget_is_250mb() {
    assert_eq!(hcs_ui::APP_RSS_BUDGET_MB, 250);
    assert_eq!(hcs_ui::IDLE_BUDGET_MB, 6144.0);
    assert_eq!(hcs_ui::PEAK_BUDGET_MB, 8192.0);
}

#[test]
fn headless_render_produces_non_blank_png() {
    // The harness must work without a display; if this test can render, so can
    // the CI job and the release gate.
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("gallery.png");
    let window = hcs_ui::HeadlessPlatform::install().expect("headless platform");
    let g = WidgetGallery::new().expect("gallery");
    hcs_ui::render_to_png(&g, &window, 720, 560, &out).expect("render");

    let img = image::open(&out).expect("png");
    let rgb = img.to_rgb8();
    assert_eq!((rgb.width(), rgb.height()), (720, 560));
    let mut seen = std::collections::HashSet::new();
    for px in rgb.pixels() {
        seen.insert(px.0);
        if seen.len() > 32 {
            break;
        }
    }
    assert!(
        seen.len() >= 8,
        "render looks blank: only {} unique colours",
        seen.len()
    );
}

#[test]
fn headless_render_rejects_zero_size() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("bad.png");
    let window = hcs_ui::HeadlessPlatform::install().expect("headless platform");
    let g = WidgetGallery::new().expect("gallery");
    let err = hcs_ui::render_to_png(&g, &window, 0, 100, &out).unwrap_err();
    assert!(matches!(err, hcs_ui::RenderError::InvalidSize { .. }));
}

#[test]
fn theme_persistence_roundtrip() {
    // Uses a temp XDG_CONFIG_HOME so the test never touches the real config.
    let dir = tempfile::tempdir().expect("tempdir");
    let prev = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    hcs_ui::save_theme(ThemePreset::Stealth).expect("save");
    assert_eq!(hcs_ui::load_theme(), ThemePreset::Stealth);
    hcs_ui::save_theme(ThemePreset::Titanium).expect("save");
    assert_eq!(hcs_ui::load_theme(), ThemePreset::Titanium);
    match prev {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
}
