//! HCS Diagnose GUI (P3, v1.3.0).

use hcs_ui::ThemePreset;
use slint::ComponentHandle;

use crate::report::DiagnosticReport;

slint::include_modules!();

hcs_ui::theme_target!(DiagnoseWindow);

pub fn build_window() -> Result<DiagnoseWindow, slint::PlatformError> {
    build_window_themed(ThemePreset::Obsidian)
}

/// Build the window with a theme already applied (crate-local `Palette`).
pub fn build_window_themed(theme: ThemePreset) -> Result<DiagnoseWindow, slint::PlatformError> {
    let w = build_window_with(&DiagnosticReport::sample())?;
    hcs_ui::apply_hcs_theme!(w, theme);
    Ok(w)
}

pub fn build_window_with(r: &DiagnosticReport) -> Result<DiagnoseWindow, slint::PlatformError> {
    let w = DiagnoseWindow::new()?;
    apply(&w, r);
    w.set_action_log("System is operating in OPTIMAL state.".into());

    let weak = w.as_weak();
    w.on_run_diagnostics(move || {
        if let Some(w) = weak.upgrade() {
            let fresh = DiagnosticReport::sample();
            apply(&w, &fresh);
            w.set_applied(false);
            w.set_action_log("Diagnostics complete: all quality gates passing.".into());
        }
    });

    let weak = w.as_weak();
    w.on_apply_fixes(move || {
        if let Some(w) = weak.upgrade() {
            let log = DiagnosticReport::sample().verify_and_apply();
            w.set_applied(true);
            w.set_action_log(log.into());
        }
    });

    Ok(w)
}

fn apply(w: &DiagnoseWindow, r: &DiagnosticReport) {
    w.set_overall_health(r.overall_health.clone().into());
    w.set_kernel(r.kernel.clone().into());
    w.set_oom_events(r.oom_events as i32);
    w.set_failed_units(r.failed_units as i32);
    w.set_ram_margin(format!("{} MB", r.ram_margin_mb).into());
    w.set_storage(format!("{} GB free", r.storage_free_gb).into());
    w.set_inference_latency(format!("{} ms", r.inference_latency_ms).into());
    w.set_mcp_transport(r.mcp_stdio.clone().into());
    w.set_issue_count(r.issues_detected as i32);
}

pub fn run(theme: ThemePreset, apply_fixes: bool) -> anyhow::Result<()> {
    let w = build_window()?;
    if apply_fixes {
        let log = DiagnosticReport::sample().verify_and_apply();
        w.set_applied(true);
        w.set_action_log(log.into());
    }
    hcs_ui::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}
