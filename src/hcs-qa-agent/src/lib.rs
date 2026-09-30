//! HCS QA Agent — the deterministic guest-side driver for VirtualBox QA.
//!
//! Gap closed from the v2 audit (§7.3). v1 captured VM screenshots by racing a
//! timer against boot, then "fixed" black frames by adding a 40-second delay.
//! That produced flaky evidence and no explanation of where a failure happened.
//!
//! This agent lives *in the image*, starts only when `hcs.qa=1` is on the kernel
//! command line, and waits on real signals instead of sleeping:
//!
//! ```text
//! host  ──▶ unix socket /run/hcs/qa.sock  ──▶ hcs-qa-agent
//!          scenario steps: wait_session · open · key · type · shot
//!                            assert_window · assert_text · rss · exit
//! ```
//!
//! Every step appends to a JSONL journal, so a failure names the step that
//! failed rather than producing an anonymous black PNG.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// One instruction from the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Step {
    /// Block until the graphical session is up. Bounded, so a hang is reported
    /// rather than waited on forever.
    WaitSession { timeout_secs: u64 },
    /// Launch an application by name.
    Open { app: String },
    /// Send a key by scancode. Kept as a name so a scenario file stays
    /// readable and survives keyboard-layout changes.
    Key { key: String },
    /// Type literal text.
    Type { text: String },
    /// Capture the screen to `name`.
    Shot { name: String },
    /// Assert that a window with this title exists.
    AssertWindow { title: String },
    /// Assert that the last capture contains this text (via OCR when available).
    AssertText { name: String, needle: String },
    /// Record the current RSS of a process, in MB.
    Rss { process: String },
    /// Sleep a bounded number of seconds. Explicit rather than implicit so every
    /// wait is visible in the journal and reviewable.
    Settle { secs: u64 },
    /// End the scenario successfully.
    Exit,
}

/// A step's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StepOutcome {
    Ok,
    /// Failed with a reason a human can act on.
    Failed {
        reason: String,
    },
    /// Ran out of time. Distinguished from `Failed` because a timeout usually
    /// means "the thing was never there", not "the thing was wrong".
    Timeout {
        secs: u64,
    },
}

impl fmt::Display for StepOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepOutcome::Ok => write!(f, "ok"),
            StepOutcome::Failed { reason } => write!(f, "failed: {reason}"),
            StepOutcome::Timeout { secs } => write!(f, "timeout after {secs}s"),
        }
    }
}

/// One journal record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub index: usize,
    pub step: Step,
    pub outcome: StepOutcome,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// Effects the runner provides for each step. Declared at module scope rather
/// than inside `Agent`: Rust does not allow nested traits, and a runner-facing
/// trait belongs next to the state machine it drives anyway.
///
/// Returning an outcome rather than a bool is deliberate: "no session after 90
/// seconds" and "the window was not there" are different failures, and the
/// journal needs to tell them apart.
pub trait Effects {
    /// Block until the graphical session is up, or time out.
    fn wait_session(&mut self, timeout_secs: u64) -> StepOutcome;
    /// Launch an application.
    fn open(&mut self, app: &str) -> StepOutcome;
    /// Deliver a named key.
    fn key(&mut self, key: &str) -> StepOutcome;
    /// Type literal text.
    fn type_text(&mut self, text: &str) -> StepOutcome;
    /// Capture the screen to `name`.
    fn shot(&mut self, name: &str) -> StepOutcome;
    /// Resident set size of a process in MB, or `None` if not running.
    fn rss_mb(&mut self, process: &str) -> Option<u64>;
    /// Wait a bounded number of seconds.
    fn settle(&mut self, secs: u64) -> StepOutcome;
}

/// The agent's state machine, separated from all I/O so every transition is
/// testable. The runner in `main` supplies the effects.
pub struct Agent {
    /// Default budget for a `WaitSession`.
    pub default_session_timeout: u64,
    /// Observed windows, injected by the runner.
    pub windows: BTreeMap<String, String>,
    /// Result of the most recent capture, for `AssertText`.
    pub last_capture_text: Option<String>,
    pub entries: Vec<JournalEntry>,
    finished: bool,
}

impl Default for Agent {
    fn default() -> Self {
        Self::new()
    }
}

impl Agent {
    pub fn new() -> Self {
        Self {
            default_session_timeout: 90,
            windows: BTreeMap::new(),
            last_capture_text: None,
            entries: Vec::new(),
            finished: false,
        }
    }

    /// Execute one step and journal it. Returns the outcome so a driver can stop
    /// early on failure rather than running steps that no longer make sense.
    pub fn run_step(&mut self, step: Step, fx: &mut dyn Effects) -> StepOutcome {
        if self.finished {
            // Steps after Exit are ignored, matching the host's expectation
            // that Exit ends the scenario.
            return StepOutcome::Failed {
                reason: "scenario already finished".to_string(),
            };
        }
        let index = self.entries.len();
        let start = std::time::Instant::now();

        let outcome = match &step {
            Step::WaitSession { timeout_secs } => fx.wait_session(*timeout_secs),
            Step::Open { app } => fx.open(app),
            Step::Key { key } => fx.key(key),
            Step::Type { text } => fx.type_text(text),
            Step::Shot { name } => fx.shot(name),
            Step::Settle { secs } => fx.settle(*secs),
            Step::Rss { process } => match fx.rss_mb(process) {
                Some(_) => StepOutcome::Ok,
                None => StepOutcome::Failed {
                    reason: format!("process '{process}' not running"),
                },
            },
            Step::AssertWindow { title } => {
                if self.windows.keys().any(|w| w.contains(title.as_str())) {
                    StepOutcome::Ok
                } else {
                    StepOutcome::Failed {
                        reason: format!("no window matching '{title}'"),
                    }
                }
            }
            Step::AssertText { name, needle } => match &self.last_capture_text {
                None => StepOutcome::Failed {
                    reason: format!("no capture available to assert '{needle}' against"),
                },
                Some(text) if text.to_lowercase().contains(&needle.to_lowercase()) => {
                    StepOutcome::Ok
                }
                Some(_) => StepOutcome::Failed {
                    reason: format!("'{needle}' not found in capture {name}"),
                },
            },
            Step::Exit => {
                self.finished = true;
                StepOutcome::Ok
            }
        };

        self.entries.push(JournalEntry {
            index,
            step,
            outcome: outcome.clone(),
            duration_ms: start.elapsed().as_millis() as u64,
        });
        outcome
    }

    /// Run a whole scenario, stopping at the first failure. Continuing past a
    /// failure produces a cascade of meaningless errors.
    pub fn run_scenario(&mut self, steps: &[Step], fx: &mut dyn Effects) -> bool {
        for step in steps {
            let outcome = self.run_step(step.clone(), fx);
            if !matches!(outcome, StepOutcome::Ok) {
                return false;
            }
        }
        true
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.entries
    }

    pub fn failed_index(&self) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.outcome != StepOutcome::Ok)
    }

    /// Render the journal as JSONL, one record per line. This is the artefact
    /// the host extracts from the guest, so it must be stable and greppable.
    pub fn journal_jsonl(&self) -> String {
        self.entries
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
}

/// A scenario parsed from a JSON file supplied by the host.
pub fn parse_scenario(text: &str) -> Result<Vec<Step>, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFx {
        session_up: bool,
        opened: Vec<String>,
        shots: Vec<String>,
        fail_shot: bool,
    }

    impl Effects for FakeFx {
        fn wait_session(&mut self, timeout_secs: u64) -> StepOutcome {
            if self.session_up {
                StepOutcome::Ok
            } else {
                StepOutcome::Timeout { secs: timeout_secs }
            }
        }
        fn open(&mut self, app: &str) -> StepOutcome {
            self.opened.push(app.to_string());
            StepOutcome::Ok
        }
        fn key(&mut self, _key: &str) -> StepOutcome {
            StepOutcome::Ok
        }
        fn type_text(&mut self, _text: &str) -> StepOutcome {
            StepOutcome::Ok
        }
        fn shot(&mut self, name: &str) -> StepOutcome {
            if self.fail_shot {
                return StepOutcome::Failed {
                    reason: "no visible surface".to_string(),
                };
            }
            self.shots.push(name.to_string());
            StepOutcome::Ok
        }
        fn rss_mb(&mut self, process: &str) -> Option<u64> {
            if process == "hcs-chat" {
                Some(11)
            } else {
                None
            }
        }
        fn settle(&mut self, _secs: u64) -> StepOutcome {
            StepOutcome::Ok
        }
    }

    fn agent_with_windows() -> Agent {
        let mut a = Agent::new();
        a.windows.insert("AI Chat".into(), "hcs-chat".into());
        a.windows.insert("Files".into(), "hcs-fm".into());
        a
    }

    #[test]
    fn scenario_runs_to_completion() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let steps = vec![
            Step::WaitSession { timeout_secs: 60 },
            Step::Open {
                app: "hcs-chat".into(),
            },
            Step::Shot {
                name: "chat".into(),
            },
            Step::Exit,
        ];
        assert!(a.run_scenario(&steps, &mut fx));
        assert_eq!(fx.opened, vec!["hcs-chat"]);
        assert_eq!(fx.shots, vec!["chat"]);
        assert_eq!(a.journal().len(), 4);
    }

    #[test]
    fn missing_session_is_a_timeout_not_a_failure() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: false,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let outcome = a.run_step(Step::WaitSession { timeout_secs: 30 }, &mut fx);
        assert_eq!(outcome, StepOutcome::Timeout { secs: 30 });
    }

    #[test]
    fn assert_window_finds_a_substring_match() {
        let mut a = agent_with_windows();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let outcome = a.run_step(Step::AssertWindow { title: "AI".into() }, &mut fx);
        assert_eq!(outcome, StepOutcome::Ok);
    }

    #[test]
    fn assert_window_reports_the_missing_title() {
        let mut a = agent_with_windows();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let outcome = a.run_step(
            Step::AssertWindow {
                title: "Nonexistent".into(),
            },
            &mut fx,
        );
        assert!(
            matches!(outcome, StepOutcome::Failed { ref reason } if reason.contains("Nonexistent"))
        );
    }

    #[test]
    fn assert_text_without_a_capture_fails_with_a_reason() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let outcome = a.run_step(
            Step::AssertText {
                name: "chat".into(),
                needle: "HCS".into(),
            },
            &mut fx,
        );
        assert!(matches!(outcome, StepOutcome::Failed { .. }));
    }

    #[test]
    fn assert_text_is_case_insensitive() {
        let mut a = Agent::new();
        a.last_capture_text = Some("hcs linux neural glass".into());
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let outcome = a.run_step(
            Step::AssertText {
                name: "boot".into(),
                needle: "HCS LINUX".into(),
            },
            &mut fx,
        );
        assert_eq!(outcome, StepOutcome::Ok);
    }

    #[test]
    fn rss_reports_missing_process() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        let outcome = a.run_step(
            Step::Rss {
                process: "not-running".into(),
            },
            &mut fx,
        );
        assert!(
            matches!(outcome, StepOutcome::Failed { ref reason } if reason.contains("not running"))
        );
    }

    #[test]
    fn scenario_stops_at_the_first_failure() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: true,
        };
        let steps = vec![
            Step::Shot {
                name: "first".into(),
            },
            Step::Shot {
                name: "second".into(),
            },
        ];
        assert!(!a.run_scenario(&steps, &mut fx));
        assert_eq!(
            fx.shots.len(),
            0,
            "the failed shot must not be recorded as taken"
        );
        assert_eq!(a.failed_index(), Some(0));
        assert_eq!(a.journal().len(), 1);
    }

    #[test]
    fn exit_finishes_the_scenario_and_ignores_later_steps() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        a.run_step(Step::Exit, &mut fx);
        assert!(a.is_finished());
        let outcome = a.run_step(
            Step::Shot {
                name: "late".into(),
            },
            &mut fx,
        );
        assert!(
            matches!(outcome, StepOutcome::Failed { ref reason } if reason.contains("already finished"))
        );
    }

    #[test]
    fn journal_jsonl_has_one_record_per_step() {
        let mut a = Agent::new();
        let mut fx = FakeFx {
            session_up: true,
            opened: vec![],
            shots: vec![],
            fail_shot: false,
        };
        a.run_step(
            Step::Open {
                app: "hcs-fm".into(),
            },
            &mut fx,
        );
        a.run_step(Step::Settle { secs: 2 }, &mut fx);
        let jsonl = a.journal_jsonl();
        assert_eq!(jsonl.lines().count(), 2);
        assert!(jsonl.contains("hcs-fm"));
    }

    #[test]
    fn scenario_round_trips_through_json() {
        let json = r#"[
            {"op":"wait_session","timeout_secs":60},
            {"op":"open","app":"hcs-chat"},
            {"op":"key","key":"Return"},
            {"op":"shot","name":"chat"},
            {"op":"exit"}
        ]"#;
        let steps = parse_scenario(json).expect("scenario must parse");
        assert_eq!(steps.len(), 5);
        assert_eq!(
            steps[1],
            Step::Open {
                app: "hcs-chat".into()
            }
        );
    }

    #[test]
    fn malformed_scenario_is_an_error_not_a_panic() {
        assert!(parse_scenario("{ not json").is_err());
        assert!(parse_scenario(r#"[{"op":"nonexistent"}]"#).is_err());
    }

    #[test]
    fn outcome_display_is_human_readable() {
        assert_eq!(StepOutcome::Ok.to_string(), "ok");
        assert_eq!(
            StepOutcome::Timeout { secs: 90 }.to_string(),
            "timeout after 90s"
        );
        assert!(StepOutcome::Failed {
            reason: "no window".into()
        }
        .to_string()
        .contains("no window"));
    }
}
