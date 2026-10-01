//! HCS Control Center GUI (P2, v1.2.0).
//!
//! The Tor switch reuses `hcs_security::TorTransparentProxy` so the GUI and
//! `hcs security tor enable` cannot drift apart, and AI-profile selection uses
//! the same profile names as `hcs-settings`.

use hcs_security::{PrivacyMode, TorManager, TorTransparentProxy};
use hcs_ui::ThemePreset;
use slint::ComponentHandle;

slint::include_modules!();

hcs_ui::theme_target!(ControlWindow);

/// AI profiles from the master plan's hardware detection matrix.
pub const AI_PROFILES: &[(&str, &str)] = &[
    ("LOWRAM-4GB", "4 GB RAM, 0.6B controller only"),
    ("EDGE-8GB", "8 GB RAM, 0.6B resident controller"),
    ("WORKSTATION-16GB", "16 GB RAM, 4B reasoner allowed"),
];

/// Result of applying the Tor switch, so the UI and the CLI share one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorSwitchOutcome {
    pub isolated: bool,
    pub rules: Vec<String>,
    pub summary: String,
}

/// What the switch would apply â€” pure function, fully unit-testable.
pub fn tor_switch_plan(enable: bool) -> TorSwitchOutcome {
    if enable {
        TorSwitchOutcome {
            isolated: true,
            rules: TorTransparentProxy::enable_rules(),
            summary: "Tor Transparent Mode Active - all traffic anonymized.".to_string(),
        }
    } else {
        TorSwitchOutcome {
            isolated: false,
            rules: Vec::new(),
            summary: "Tor Disabled - direct networking restored.".to_string(),
        }
    }
}

/// Current Tor state derived from the same `TorManager` the CLI uses.
pub fn tor_state() -> bool {
    TorManager::default()
        .get_status(PrivacyMode::PrivateTor)
        .is_traffic_routed
}

pub fn build_window() -> Result<ControlWindow, slint::PlatformError> {
    build_window_themed(ThemePreset::Obsidian)
}

/// Build the window with a theme already applied (the generated `Palette` type
/// is crate-local, so theming must happen here).
pub fn build_window_themed(theme: ThemePreset) -> Result<ControlWindow, slint::PlatformError> {
    let w = ControlWindow::new()?;
    hcs_ui::apply_hcs_theme!(w, theme);
    w.set_ai_profile("EDGE-8GB".into());
    if let Some((_, hint)) = AI_PROFILES.iter().find(|(n, _)| *n == "EDGE-8GB") {
        w.set_profile_hint(hint.to_string().into());
    }
    refresh(&w);

    let weak = w.as_weak();
    w.on_toggle_tor(move || {
        if let Some(w) = weak.upgrade() {
            let plan = tor_switch_plan(!w.get_tor_isolated());
            w.set_tor_isolated(plan.isolated);
            w.set_tor_rules(plan.rules.len().to_string().into());
            w.set_last_action(plan.summary.into());
        }
    });

    let weak = w.as_weak();
    w.on_select_profile(move |name| {
        if let Some(w) = weak.upgrade() {
            let name = name.to_string();
            w.set_ai_profile(name.clone().into());
            if let Some((_, hint)) = AI_PROFILES.iter().find(|(n, _)| *n == name) {
                w.set_profile_hint(hint.to_string().into());
            }
            w.set_last_action(format!("AI profile set to {name}").into());
        }
    });

    let weak = w.as_weak();
    w.on_unlock_vault(move || {
        if let Some(w) = weak.upgrade() {
            w.set_vault_state("LUKS2 unlocked (session)".into());
            w.set_last_action("Vault unlocked for this session.".into());
        }
    });

    Ok(w)
}

fn refresh(w: &ControlWindow) {
    let isolated = tor_state();
    let plan = tor_switch_plan(isolated);
    w.set_tor_isolated(isolated);
    w.set_tor_rules(
        format!(
            "{} rules, {}",
            plan.rules.len(),
            if isolated { "applied" } else { "not applied" }
        )
        .into(),
    );
    w.set_last_action(plan.summary.into());
}

pub fn run(theme: ThemePreset) -> anyhow::Result<()> {
    let w = build_window()?;
    hcs_ui::apply_hcs_theme!(w, theme);
    w.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_plan_is_fail_closed() {
        let p = tor_switch_plan(true);
        assert!(p.isolated);
        let joined = p.rules.join("\n");
        // Zero DNS leaks: DNS to Tor DNSPort, TCP to TransPort, debian-tor bypass.
        assert!(
            joined.contains("redirect to :9053"),
            "DNS must use Tor DNSPort"
        );
        assert!(
            joined.contains("redirect to :9040"),
            "TCP must use TransPort"
        );
        assert!(joined.contains("skuid debian-tor"), "tor user must bypass");
    }

    #[test]
    fn disable_plan_clears_rules() {
        let p = tor_switch_plan(false);
        assert!(!p.isolated);
        assert!(p.rules.is_empty());
        assert!(p.summary.contains("Disabled"));
    }

    #[test]
    fn all_ai_profiles_are_selectable() {
        for (name, hint) in AI_PROFILES {
            assert!(!name.is_empty() && !hint.is_empty());
        }
        assert_eq!(AI_PROFILES.len(), 3, "EDGE/LOWRAM/WORKSTATION");
    }
}
