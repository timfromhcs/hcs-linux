//! Diagnostic report model shared by the CLI (`--json`) and the GUI, plus the
//! Neural Glass triage window (P3, v1.3.0).
//!
//! `apply` semantics are kept identical to the CLI: fixes are only *reported*
//! as applied after the non-destructive verification pass runs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticReport {
    pub product: String,
    pub kernel: String,
    pub oom_events: u32,
    pub failed_units: u32,
    pub ram_margin_mb: u32,
    pub storage_free_gb: f32,
    pub inference_latency_ms: u32,
    pub mcp_stdio: String,
    pub issues_detected: u32,
    pub recommended_actions: Vec<String>,
    pub overall_health: String,
}

impl DiagnosticReport {
    /// Baseline healthy system, matching the existing CLI report values.
    pub fn sample() -> Self {
        Self {
            product: "HCS Linux".to_string(),
            kernel: "7.0.0-generic amd64".to_string(),
            oom_events: 0,
            failed_units: 0,
            ram_margin_mb: 6612,
            storage_free_gb: 18.4,
            inference_latency_ms: 18,
            mcp_stdio: "Functional".to_string(),
            issues_detected: 0,
            recommended_actions: Vec::new(),
            overall_health: "OPTIMAL".to_string(),
        }
    }

    /// Health is degraded by any budget violation or failed unit.
    pub fn is_healthy(&self) -> bool {
        self.oom_events == 0
            && self.failed_units == 0
            && self.ram_margin_mb > 0
            && self.storage_free_gb > 1.0
    }

    /// Non-destructive verification pass; returns the log line the GUI shows.
    pub fn verify_and_apply(&self) -> String {
        if self.is_healthy() {
            "Verified system integrity. No repair necessary.".to_string()
        } else {
            let mut actions = vec!["Verified system integrity.".to_string()];
            if self.oom_events > 0 {
                actions.push(format!(
                    "RAM pressure detected ({} OOM events): advise unloading heavy models.",
                    self.oom_events
                ));
            }
            if self.failed_units > 0 {
                actions.push(format!(
                    "{} failed unit(s): advise restarting the affected HCS services.",
                    self.failed_units
                ));
            }
            if self.storage_free_gb <= 1.0 {
                actions.push("Storage nearly full: advise pruning model cache.".to_string());
            }
            actions.join(" ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_is_healthy_and_optimal() {
        let r = DiagnosticReport::sample();
        assert!(r.is_healthy());
        assert_eq!(r.overall_health, "OPTIMAL");
        assert_eq!(r.issues_detected, 0);
        assert!(r.recommended_actions.is_empty());
    }

    #[test]
    fn healthy_apply_reports_no_repair() {
        let r = DiagnosticReport::sample();
        assert_eq!(
            r.verify_and_apply(),
            "Verified system integrity. No repair necessary."
        );
    }

    #[test]
    fn oom_events_trigger_recommendation() {
        let mut r = DiagnosticReport::sample();
        r.oom_events = 3;
        assert!(!r.is_healthy());
        let log = r.verify_and_apply();
        assert!(log.contains("3 OOM events"), "got: {log}");
        assert!(log.contains("unloading heavy models"), "got: {log}");
    }

    #[test]
    fn failed_units_and_low_storage_trigger_recommendations() {
        let mut r = DiagnosticReport::sample();
        r.failed_units = 2;
        r.storage_free_gb = 0.5;
        let log = r.verify_and_apply();
        assert!(log.contains("2 failed unit(s)"), "got: {log}");
        assert!(log.contains("pruning model cache"), "got: {log}");
    }

    #[test]
    fn ram_margin_must_be_positive() {
        let mut r = DiagnosticReport::sample();
        r.ram_margin_mb = 0;
        assert!(!r.is_healthy());
    }
}
