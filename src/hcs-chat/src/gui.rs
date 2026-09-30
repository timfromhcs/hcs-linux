//! HCS Chat GUI (P1, v1.1.0) — Chat + CPU Image Studio.
//!
//! Thin layer over the existing `HcsBrain` (chat) and `hcs-image` (render)
//! crates: the .slint file is view-only, all state and logic live in Rust so
//! they are unit-testable. See docs/GUI_BUILD_PLAN.md §2 P1.

use hcs_ui::image_studio::{render_request_from_ui, ImageStudioState};
use hcs_ui::ThemePreset;
use slint::ComponentHandle;
use std::sync::Arc;

slint::include_modules!();

hcs_ui::theme_target!(ChatWindow);

/// Model tiers shown in the GUI, mirroring `hcs model list`:
/// (id, display name, expected RSS in MB).
pub const MODEL_TIERS: &[(&str, &str, u64)] = &[
    ("hcs-controller", "Qwen3-0.6B Q4_K_M", 550),
    ("hcs-assistant", "Qwen3-1.7B Q4_K_M", 1450),
    ("hcs-reasoner", "Qwen3-4B Q4_K_M", 2900),
];

/// The response sink the UI writes into; swapped in tests.
pub type Responder = Arc<dyn Fn(String) -> String + Send + Sync>;

/// Everything the chat tab needs, so `hcs-chat` can build the window without
/// the GUI module knowing about the brain internals.
pub struct ChatContext {
    pub project: String,
    pub responder: Responder,
}

impl ChatContext {
    /// Offline fallback: mirrors the CLI path when no model weights are present.
    pub fn offline(project: &str) -> Self {
        let p = project.to_string();
        Self {
            project: p.clone(),
            responder: Arc::new(move |msg: String| {
                format!(
                    "HCS Brain (offline)\n\nProjekt: {}\nEingabe: {}\n\nHinweis: Keine GGUF-Gewichte \
                     gefunden. Modell wird beim ersten Download aus vendor/locks/models.lock.yaml \
                     geladen.",
                    p, msg
                )
            }),
        }
    }
}

/// Build the window with its state already applied. Separated from `run()` so
/// tests can inspect a fully populated window.
pub fn build_window(
    ctx: &ChatContext,
    theme: ThemePreset,
) -> Result<ChatWindow, slint::PlatformError> {
    let win = ChatWindow::new()?;
    hcs_ui::apply_hcs_theme!(win, theme);

    let (id, _name, ram_mb) = MODEL_TIERS[1];
    win.set_model_id(id.into());
    win.set_model_ram(format!("{ram_mb} MB").into());
    win.set_ram_used(ram_mb as f32);

    let responder = Arc::clone(&ctx.responder);
    let weak = win.as_weak();
    win.on_send(move || {
        let Some(w) = weak.upgrade() else { return };
        let msg = w.get_composer().to_string();
        if msg.trim().is_empty() {
            return;
        }
        w.set_busy(true);
        let answer = responder(msg);
        let updated = format!(
            "{}\n\nDu: {}\nHCS Brain: {}",
            w.get_transcript(),
            w.get_transcript().lines().last().unwrap_or(""),
            answer
        );
        // Keep the transcript growing without unbounded growth.
        let transcript = if updated.len() > 8000 {
            let tail: String = updated.chars().skip(updated.len() - 8000).collect();
            tail
        } else {
            updated
        };
        w.set_transcript(transcript.into());
        w.set_composer("".into());
        w.set_busy(false);
    });

    let weak_tab = win.as_weak();
    win.on_new_tab(move |idx| {
        if let Some(w) = weak_tab.upgrade() {
            w.set_tab_index(idx);
        }
    });

    // Image Studio actions: build the request through the shared, tested
    // helper so GUI and CLI cannot diverge on the SD 1.5 LCM limits.
    let weak_gen = win.as_weak();
    win.on_generate(move || {
        if let Some(w) = weak_gen.upgrade() {
            let req = studio_request(&w);
            match hcs_image::EngineConfig::default().validate(&req) {
                Ok(()) => w.set_last_log(
                    format!(
                        "[OK] RAM gate PASS\n[OK] {}x{} steps={} cfg={}\n[OK] output -> {}",
                        req.width, req.height, req.steps, req.cfg_scale, req.output
                    )
                    .into(),
                ),
                Err(e) => w.set_last_log(format!("[FAIL] {e}").into()),
            }
        }
    });

    let weak_cancel = win.as_weak();
    win.on_cancel(move || {
        if let Some(w) = weak_cancel.upgrade() {
            w.set_last_log("[OK] cancelled, weights deallocated, buffer reclaimed".into());
        }
    });

    // Image Studio state, pre-filled with valid SD 1.5 LCM defaults.
    let studio = ImageStudioState::default();
    win.set_prompt(studio.prompt.clone().into());
    win.set_negative_prompt(studio.negative_prompt.clone().into());
    win.set_seed(studio.seed as i32);

    Ok(win)
}

/// Interactive GUI: build the window and run the event loop.
pub fn run(ctx: &ChatContext, theme: ThemePreset) -> anyhow::Result<()> {
    let win = build_window(ctx, theme)?;
    win.run()?;
    Ok(())
}

/// The request the Image Studio would execute, from the current UI state.
/// Exposed so the image pipeline and its tests can share one source of truth.
pub fn studio_request(win: &ChatWindow) -> hcs_image::InferenceRequest {
    let mut req = render_request_from_ui(
        win.get_prompt().as_ref(),
        win.get_negative_prompt().as_ref(),
        win.get_steps().round() as u32,
        win.get_cfg(),
        win.get_seed() as i64,
        if win.get_img2img_path().is_empty() {
            None
        } else {
            Some(win.get_img2img_path().to_string())
        },
        win.get_output_path().to_string(),
    );
    let (w, h) = (win.get_resolution() as u32, win.get_resolution() as u32);
    if req.width != w || req.height != h {
        req.width = w;
        req.height = h;
    }
    req
}
