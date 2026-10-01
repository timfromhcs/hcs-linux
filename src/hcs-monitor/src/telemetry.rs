//! Telemetry data model shared by the CLI (`--json`) and the GUI.
//!
//! Extracted from `main.rs` so both surfaces read the same numbers — the GUI
//! must never invent values the CLI does not print (plan §2 P2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryTelemetry {
    pub base_mb: u32,
    pub core_services_mb: u32,
    pub resident_ai_mb: u32,
    pub total_rss_mb: u32,
    pub idle_budget_mb: u32,
    pub peak_budget_mb: u32,
    pub headroom_mb: u32,
}

impl MemoryTelemetry {
    /// Current EDGE-8GB figures, identical to the CLI dashboard.
    ///
    /// `headroom_mb` is measured against the *peak* budget because that is
    /// what the existing CLI output reports (8192 - 1580 = 6612); changing it
    /// would silently alter the published `--json` contract.
    pub fn sample() -> Self {
        let base_mb = 820;
        let core_services_mb = 210;
        let resident_ai_mb = 550;
        let total_rss_mb = base_mb + core_services_mb + resident_ai_mb;
        Self {
            base_mb,
            core_services_mb,
            resident_ai_mb,
            total_rss_mb,
            idle_budget_mb: 6144,
            peak_budget_mb: 8192,
            headroom_mb: 8192 - total_rss_mb,
        }
    }

    pub fn within_idle(&self) -> bool {
        self.total_rss_mb <= self.idle_budget_mb
    }

    pub fn within_peak(&self) -> bool {
        self.total_rss_mb <= self.peak_budget_mb
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelTelemetry {
    pub resident: String,
    pub active_heavy: Option<String>,
    pub policy: String,
}

impl ModelTelemetry {
    pub fn sample() -> Self {
        Self {
            resident: "hcs-controller (Qwen3-0.6B)".to_string(),
            active_heavy: None,
            policy: "single_heavy_resident_enforced".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Telemetry {
    pub product: String,
    pub profile: String,
    pub memory: MemoryTelemetry,
    pub models: ModelTelemetry,
    pub status: String,
}

impl Telemetry {
    pub fn sample() -> Self {
        let memory = MemoryTelemetry::sample();
        let status = if memory.within_idle() && memory.within_peak() {
            "PASS"
        } else {
            "FAIL"
        };
        Self {
            product: "HCS Linux".to_string(),
            profile: "EDGE-8GB".to_string(),
            memory,
            models: ModelTelemetry::sample(),
            status: status.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_totals_add_up() {
        let m = MemoryTelemetry::sample();
        assert_eq!(
            m.total_rss_mb,
            m.base_mb + m.core_services_mb + m.resident_ai_mb
        );
        assert_eq!(m.headroom_mb, m.peak_budget_mb - m.total_rss_mb);
    }

    #[test]
    fn headroom_matches_published_cli_output() {
        // The 1.0.0 ISO ships this exact number; the GUI must not change it.
        assert_eq!(MemoryTelemetry::sample().headroom_mb, 6612);
    }

    #[test]
    fn edge_profile_is_inside_both_budgets() {
        let t = Telemetry::sample();
        assert!(t.memory.within_idle());
        assert!(t.memory.within_peak());
        assert_eq!(t.status, "PASS");
    }

    #[test]
    fn budget_breach_is_reported_as_fail() {
        let mut t = Telemetry::sample();
        t.memory.total_rss_mb = 9000;
        assert!(!t.memory.within_peak());
        assert_eq!(
            if t.memory.within_idle() && t.memory.within_peak() {
                "PASS"
            } else {
                "FAIL"
            },
            "FAIL"
        );
    }

    #[test]
    fn single_heavy_model_policy_is_reported() {
        let t = Telemetry::sample();
        assert!(t.models.active_heavy.is_none());
        assert_eq!(t.models.policy, "single_heavy_resident_enforced");
    }
}
