//! hcs-chat GUI tests (P1).
//!
//! Slint installs its platform once per *process/thread*, while libtest spawns
//! a thread per `#[test]`. Creating a window from two different test threads
//! therefore fails with "The Slint platform was initialized in another
//! thread". So all window interaction lives in ONE test (`chat_window`) and
//! runs sequentially; the pure-logic tests (no window) stay independent and
//! parallel. Rendering is covered separately by `scripts/verify_gui.py`, which
//! renders through the standalone `hcs-gui-render` binary.
//!
//! The window is created on the *headless* platform, not winit. `hcs-chat`
//! links `backend-winit`, so on a build machine with no display the default
//! platform aborts with "neither WAYLAND_DISPLAY nor DISPLAY is set" — which
//! is what CI hit. Installing the software-renderer platform first makes the
//! test behave identically on a workstation and on a runner.

use hcs_ui::ThemePreset;

fn window() -> hcs_chat::gui::ChatWindow {
    hcs_ui::install_headless().expect("headless platform");
    let ctx = hcs_chat::gui::ChatContext::offline("test-project");
    hcs_chat::gui::build_window(&ctx, ThemePreset::Obsidian).expect("chat window")
}

#[test]
fn chat_window() {
    // --- defaults
    let w = window();
    assert_eq!(w.get_model_id().as_str(), "hcs-assistant");
    assert_eq!(w.get_model_ram().as_str(), "1450 MB");
    assert_eq!(w.get_theme_name().as_str(), "obsidian");
    assert_eq!(w.get_tab_index(), 0);
    assert!(!w.get_busy());
    assert_eq!(w.get_steps() as i32, 6, "LCM default 6 steps");
    assert!((1.5..=2.0).contains(&w.get_cfg()), "CFG within plan range");
    assert_eq!(
        w.get_prompt().as_str(),
        "a sleek futuristic cybernetic workstation in neon glass"
    );
    assert!(w.get_negative_prompt().as_str().contains("blurry"));

    // --- tab switching
    w.invoke_new_tab(1);
    assert_eq!(w.get_tab_index(), 1, "Image Studio tab selectable");
    w.invoke_new_tab(0);
    assert_eq!(w.get_tab_index(), 0, "back to Chat");

    // --- empty message is ignored
    let before = w.get_transcript().to_string();
    w.set_composer("   ".into());
    w.invoke_send();
    assert_eq!(w.get_transcript().to_string(), before, "blank send ignored");

    // --- send appends and clears
    w.set_composer("wie läuft der RAM-Status?".into());
    w.invoke_send();
    let after = w.get_transcript().to_string();
    assert!(after.len() > before.len(), "transcript must grow");
    assert!(
        after.contains("wie läuft der RAM-Status?"),
        "user turn recorded"
    );
    assert!(
        after.contains("Projekt: test-project"),
        "responder used project"
    );
    assert_eq!(w.get_composer().as_str(), "", "composer cleared");
    assert!(!w.get_busy(), "busy flag released");

    // --- studio request clamping (same window, no new platform)
    w.set_steps(99.0);
    w.set_cfg(12.0);
    let req = hcs_chat::gui::studio_request(&w);
    assert_eq!(req.steps, 8, "steps clamped to LCM max");
    assert_eq!(req.cfg_scale, 2.5, "cfg clamped to max");
    assert_eq!((req.width, req.height), (512, 512));

    w.set_img2img_path("/tmp/base.png".into());
    let req = hcs_chat::gui::studio_request(&w);
    assert_eq!(req.mode, hcs_image::ImageMode::Img2Img);
    assert_eq!(req.input_image.as_deref(), Some("/tmp/base.png"));
}

#[test]
fn theme_presets_map_to_exclusive_flags() {
    // No window needed: pure mapping logic.
    assert_eq!(
        hcs_ui::preset_flags(ThemePreset::Obsidian),
        (false, false, false)
    );
    assert_eq!(
        hcs_ui::preset_flags(ThemePreset::Titanium),
        (true, false, false)
    );
    assert_eq!(
        hcs_ui::preset_flags(ThemePreset::Stealth),
        (false, true, false)
    );
    assert_eq!(
        hcs_ui::preset_flags(ThemePreset::HighContrast),
        (false, false, true)
    );
    // Mutual exclusivity is the invariant the window relies on: two flags set at
    // once would resolve the palette to an arbitrary preset. Obsidian is the
    // default, so it is expressed as "no flag set" rather than a third flag.
    for p in ThemePreset::all() {
        let (t, s, h) = hcs_ui::preset_flags(*p);
        assert!(
            [t, s, h].iter().filter(|f| **f).count() <= 1,
            "{} must not enable more than one theme flag",
            p.as_str()
        );
    }
    assert_eq!(
        hcs_ui::preset_flags(ThemePreset::Obsidian),
        (false, false, false)
    );
}
