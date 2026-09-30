//! HCS Monitor GUI (P2, v1.2.0). All numbers come from `Telemetry::sample()`,
//! the same source the CLI `--json` output uses.

use hcs_ui::ThemePreset;
use slint::ComponentHandle;

use crate::telemetry::Telemetry;

slint::include_modules!();

hcs_ui::theme_target!(MonitorWindow);

/// Build the monitor window populated from the telemetry model.
pub fn build_window() -> Result<MonitorWindow, slint::PlatformError> {
    build_window_themed(ThemePreset::Obsidian)
}

/// Build the window with a theme already applied. The generated `Palette` type
/// is crate-local, so theming must happen here rather than in the render tool.
pub fn build_window_themed(theme: ThemePreset) -> Result<MonitorWindow, slint::PlatformError> {
    let w = build_window_with(&Telemetry::sample())?;
    hcs_ui::apply_hcs_theme!(w, theme);
    Ok(w)
}

/// Build the window from an explicit telemetry snapshot (used by tests).
pub fn build_window_with(t: &Telemetry) -> Result<MonitorWindow, slint::PlatformError> {
    let w = MonitorWindow::new()?;
    apply(&w, t);
    let weak = w.as_weak();
    w.on_refresh(move || {
        if let Some(w) = weak.upgrade() {
            apply(&w, &Telemetry::sample());
        }
    });
    Ok(w)
}

fn apply(w: &MonitorWindow, t: &Telemetry) {
    w.set_profile(t.profile.clone().into());
    w.set_base_mb(t.memory.base_mb as f32);
    w.set_services_mb(t.memory.core_services_mb as f32);
    w.set_resident_ai_mb(t.memory.resident_ai_mb as f32);
    w.set_total_rss_mb(t.memory.total_rss_mb as f32);
    w.set_headroom_mb(t.memory.headroom_mb as f32);
    w.set_resident_model(t.models.resident.clone().into());
    w.set_resident_ram(format!("{} MB", t.memory.resident_ai_mb).into());
    w.set_heavy_model(
        t.models
            .active_heavy
            .clone()
            .unwrap_or_else(|| "none (single-resident enforced)".to_string())
            .into(),
    );
    w.set_status(t.status.clone().into());
}

/// Interactive GUI.
pub fn run(theme: ThemePreset) -> anyhow::Result<()> {
    let w = build_window()?;
    hcs_ui::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}
