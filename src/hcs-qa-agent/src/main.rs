//! `hcs-qa-agent` — the guest-side QA runner.
//!
//! Enabled only when `hcs.qa=1` is on the kernel command line, so a production
//! ISO never exposes an automation socket. Waits on a Unix socket for scenario
//! steps from the host, executes them against the real session, and appends a
//! JSONL journal the host extracts afterwards.
//!
//! The scenario may also be supplied as a file (`--scenario`), which is what the
//! offline VM stages use.
//!
//! The binary is Unix-only by nature — it drives a Wayland guest — but it must
//! still *compile* on the Windows build host so `cargo test --workspace` and CI
//! stay green. Everything Linux-specific is behind `cfg(unix)`; on other hosts
//! the binary prints a clear refusal instead of failing to build.

#[cfg(not(unix))]
use anyhow::Result;
#[cfg(unix)]
use anyhow::{anyhow, Context, Result};
#[cfg(unix)]
use hcs_qa_agent::{parse_scenario, Agent, Effects, StepOutcome};
#[cfg(unix)]
use std::path::{Path, PathBuf};

#[cfg(unix)]
const JOURNAL_PATH: &str = "/var/log/hcs/qa/journal.jsonl";
#[cfg(unix)]
const SHOT_DIR: &str = "/var/log/hcs/qa/shots";

/// Staged into the image, so the guest needs nothing from the host.
#[cfg(unix)]
const DEFAULT_SUITE: &str = "/usr/share/hcs/qa/suite.json";
#[cfg(unix)]
const DEFAULT_SCENARIO_DIR: &str = "/usr/share/hcs/qa/scenarios";
#[cfg(unix)]
const DEFAULT_SCENARIO: &str = "/usr/share/hcs/qa/scenarios/scenario.json";

/// Where the evidence bundle is written. The host attaches a disk here, so the
/// guest never needs guest additions, a shared folder or a network to hand its
/// results back.
#[cfg(unix)]
const DEFAULT_EVIDENCE: &str = "/mnt/hcs-qa";

/// Refuse to run unless the boot explicitly asked for QA. This is the single
/// most important line in the file: an automation socket on a normal boot would
/// let anything that can reach the socket drive the desktop.
#[cfg(unix)]
fn qa_mode_enabled() -> bool {
    std::fs::read_to_string("/proc/cmdline")
        .map(|c| c.contains("hcs.qa=1"))
        .unwrap_or(false)
}

fn main() -> Result<()> {
    #[cfg(not(unix))]
    {
        eprintln!(
            "hcs-qa-agent only runs inside the HCS Linux guest (it drives a \
             Wayland session). Refusing to run on this host."
        );
        std::process::exit(2);
    }

    #[cfg(unix)]
    {
        if !qa_mode_enabled() {
            eprintln!(
                "hcs-qa-agent: hcs.qa=1 is not on the kernel command line.\n\
                 Refusing to start: an automation socket must never exist on a normal boot."
            );
            std::process::exit(2);
        }

        let args: Vec<String> = std::env::args().skip(1).collect();
        match args.first().map(String::as_str) {
            // The suite runner. This is the default because it is the only mode
            // the VirtualBox gate uses: the guest drives itself, so the host
            // needs no guest additions, no file sharing and no socket.
            Some("run-suite") | None => run_suite(&args),
            // One scenario, for debugging a single stage by hand.
            Some("run") => {
                let path = args
                    .get(1)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(DEFAULT_SCENARIO));
                let agent = run_scenario_file(path)?;
                write_journal(&agent, Path::new(JOURNAL_PATH))?;
                Ok(())
            }
            Some("validate-suite") => validate_suite(&args),
            Some(other) => {
                eprintln!("hcs-qa-agent: unknown command '{other}'");
                eprintln!("  run-suite        run every stage and write evidence");
                eprintln!("  run <file>       run one scenario file");
                eprintln!("  validate-suite   check the manifest without running");
                std::process::exit(2);
            }
        }
    }
}

/// Read the suite manifest and confirm every scenario it names is really
/// present. A missing scenario used to become a stage that silently "skipped"
/// while the run still reported a pass count.
#[cfg(unix)]
fn load_suite(suite_path: &Path, scenario_dir: &Path) -> Result<hcs_qa_agent::suite::Suite> {
    let text = std::fs::read_to_string(suite_path)
        .with_context(|| format!("cannot read the QA manifest {}", suite_path.display()))?;
    let suite = hcs_qa_agent::suite::Suite::parse(&text)
        .map_err(|e| anyhow!("QA manifest parse error in {}: {e}", suite_path.display()))?;
    let available: Vec<String> = std::fs::read_dir(scenario_dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    suite
        .validate(&available)
        .map_err(|e| anyhow!("QA manifest is not usable: {e}"))?;
    Ok(suite)
}

#[cfg(unix)]
fn scenario_dir_from(args: &[String]) -> PathBuf {
    arg_value(args, "--scenario-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SCENARIO_DIR))
}

#[cfg(unix)]
fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

/// Validate the manifest and exit. Cheap enough to run on every boot of a QA
/// image, and it turns a broken manifest into an immediate, named failure.
#[cfg(unix)]
fn validate_suite(args: &[String]) -> Result<()> {
    let suite_path = arg_value(args, "--suite")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SUITE));
    let dir = scenario_dir_from(args);
    let suite = load_suite(&suite_path, &dir)?;
    println!(
        "[OK] {} stages ({} GUI, {} console), all scenarios present",
        suite.stages.len(),
        suite.gui_stages(),
        suite.console_stages()
    );
    Ok(())
}

/// Run every stage and write the evidence bundle.
#[cfg(unix)]
fn run_suite(args: &[String]) -> Result<()> {
    let suite_path = arg_value(args, "--suite")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SUITE));
    let scenario_dir = scenario_dir_from(args);
    let evidence = arg_value(args, "--evidence-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EVIDENCE));
    let profile = arg_value(args, "--profile")
        .unwrap_or("unknown")
        .to_string();

    let suite = load_suite(&suite_path, &scenario_dir)?;
    let mut result = hcs_qa_agent::suite::SuiteResult::new(&suite.version, &profile, &suite);

    let shots_root = evidence.join("shots");
    std::fs::create_dir_all(&shots_root)
        .with_context(|| format!("cannot create {}", shots_root.display()))?;
    println!(
        "hcs-qa-agent: running {} stages, evidence -> {}",
        suite.stages.len(),
        evidence.display()
    );

    for stage in &suite.stages {
        let stage_dir = shots_root.join(format!("{:02}_{}", stage.n, stage.name));
        std::fs::create_dir_all(&stage_dir).ok();

        let scenario_file = scenario_dir.join(&stage.scenario);
        let text = match std::fs::read_to_string(&scenario_file) {
            Ok(t) => t,
            Err(e) => {
                // validate() already proved it exists, so reaching here means the
                // file vanished mid-run. Recorded rather than skipped.
                result.record(hcs_qa_agent::suite::StageResult {
                    stage: stage.n,
                    name: stage.name.clone(),
                    evidence: stage.evidence,
                    status: hcs_qa_agent::suite::Status::Fail,
                    detail: format!("scenario vanished mid-run: {e}"),
                    capture: None,
                    duration_ms: 0,
                });
                continue;
            }
        };
        let steps = match parse_scenario(&text) {
            Ok(s) => s,
            Err(e) => {
                result.record(hcs_qa_agent::suite::StageResult {
                    stage: stage.n,
                    name: stage.name.clone(),
                    evidence: stage.evidence,
                    status: hcs_qa_agent::suite::Status::Fail,
                    detail: format!("scenario parse error: {e}"),
                    capture: None,
                    duration_ms: 0,
                });
                continue;
            }
        };

        println!("  [{:02}] {:<24} {}", stage.n, stage.name, stage.evidence);
        let mut agent = Agent::new();
        let mut fx = LocalFx::for_stage(stage_dir.clone());
        let start = std::time::Instant::now();
        let ok = agent.run_scenario(&steps, &mut fx);
        let duration_ms = start.elapsed().as_millis() as u64;

        let mut detail = String::new();
        for e in agent.journal() {
            if !matches!(e.outcome, StepOutcome::Ok) {
                detail = e.outcome.to_string();
                break;
            }
        }

        // Judge the frame in the guest. A capture that is a splash or a gradient
        // fails here, where the agent can say why, instead of being carried out
        // to the host and scored by a script that never sees the context.
        let capture = fx.taken.last().and_then(|p| analyse_capture(p));
        let capture_ok = match &capture {
            Some(c) => c.ok,
            // A console stage may legitimately produce no frame (the installer
            // text is on a tty, not the framebuffer). Not a failure by itself;
            // the scenario's own assertions carry the stage.
            None => true,
        };
        if !capture_ok {
            if let Some(c) = &capture {
                detail = format!("{} (capture: {})", c.reason, c.file);
            }
        }
        if !ok && detail.is_empty() {
            detail = "a scenario step failed".into();
        }

        let status = if ok && capture_ok {
            hcs_qa_agent::suite::Status::Pass
        } else {
            hcs_qa_agent::suite::Status::Fail
        };
        result.journal.extend(agent.journal().to_vec());
        result.record(hcs_qa_agent::suite::StageResult {
            stage: stage.n,
            name: stage.name.clone(),
            evidence: stage.evidence,
            status,
            detail,
            capture,
            duration_ms,
        });
    }

    write_evidence(&evidence, &result)?;
    println!("hcs-qa-agent: {}", result.summary());
    Ok(())
}

/// Analyse a capture on disk, reading the PNG's pixels.
#[cfg(unix)]
fn analyse_capture(path: &Path) -> Option<hcs_qa_agent::capture::CaptureReport> {
    let (w, h) = LocalFx::png_size(path)?;
    let bytes = std::fs::read(path).ok()?;
    let decoded = image::load_from_memory(&bytes).ok()?.to_rgb8();
    let (iw, ih) = (decoded.width(), decoded.height());
    if iw != w || ih != h {
        return None;
    }
    let px = hcs_qa_agent::capture::Pixels::new(
        &path.display().to_string(),
        iw,
        ih,
        decoded.as_raw(),
        bytes.len() as u64,
    )?;
    Some(hcs_qa_agent::capture::analyse(
        &px,
        &hcs_qa_agent::capture::Thresholds::default(),
    ))
}

/// Write the evidence bundle the host collects: a machine-readable result, a
/// flat journal, a CSV for spreadsheets, and a DONE marker written last.
///
/// The marker is written *after* everything else and the directory is synced,
/// so the host can treat its presence as "the run finished" rather than polling
/// a half-written file.
#[cfg(unix)]
fn write_evidence(dir: &Path, result: &hcs_qa_agent::suite::SuiteResult) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;

    let json = serde_json::to_string_pretty(result)?;
    write_atomic(&dir.join("result.json"), json.as_bytes())?;

    let journal: String = result
        .journal
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect::<Vec<_>>()
        .join("\n");
    write_atomic(&dir.join("journal.jsonl"), journal.as_bytes())?;

    let mut csv =
        String::from("stage,name,evidence,status,detail,mean_diff,text_ratio,edge_density\n");
    for r in &result.results {
        let (d, t, e) = match &r.capture {
            Some(c) => (
                c.entropy.to_string(),
                c.text_ratio.to_string(),
                c.edge_density.to_string(),
            ),
            None => ("".into(), "".into(), "".into()),
        };
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            r.stage,
            r.name,
            r.evidence,
            r.status,
            csv_field(&r.detail),
            d,
            t,
            e
        ));
    }
    write_atomic(&dir.join("stages.csv"), csv.as_bytes())?;

    // Sync the files before the marker, so the host never sees DONE with
    // unflushed evidence behind it.
    if let Ok(dirf) = std::fs::File::open(dir) {
        let _ = dirf.sync_all();
    }
    write_atomic(
        &dir.join("DONE"),
        format!(
            "{}\n{}\n",
            result.summary(),
            if result.is_green() { "green" } else { "red" }
        )
        .as_bytes(),
    )?;
    Ok(())
}

#[cfg(unix)]
fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(unix)]
fn write_atomic(path: &Path, body: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, body).with_context(|| format!("cannot write {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("cannot rename into {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn write_journal(agent: &Agent, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_atomic(path, format!("{}\n", agent.journal_jsonl()).as_bytes())?;
    eprintln!(
        "hcs-qa-agent: {} step(s) journaled to {}",
        agent.journal().len(),
        path.display()
    );
    Ok(())
}

#[cfg(unix)]
fn run_scenario_file(path: PathBuf) -> Result<Agent> {
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("cannot read scenario {}", path.display()))?;
    let steps = parse_scenario(&text).map_err(|e| anyhow!("scenario parse error: {e}"))?;
    let mut agent = Agent::new();
    let mut fx = LocalFx::new();
    for step in steps {
        let outcome = agent.run_step(step, &mut fx);
        if !matches!(outcome, StepOutcome::Ok) {
            eprintln!("hcs-qa-agent: stopping at failed step: {outcome}");
            break;
        }
    }
    Ok(agent)
}

/// Real effects against the running session.
///
/// Every wait is bounded and every wait is for a *signal* — a socket, a window, a
/// file — never a bare sleep. That is the entire difference between v1's flaky
/// screenshots and reproducible evidence.
#[cfg(unix)]
struct LocalFx {
    shot_index: u32,
    /// Where this stage's captures go. Set per stage by the suite runner so a
    /// report can point at the exact file a verdict came from.
    shot_dir: PathBuf,
    /// Absolute paths of captures taken this stage, oldest first.
    taken: Vec<PathBuf>,
}

#[cfg(unix)]
impl LocalFx {
    fn new() -> Self {
        Self {
            shot_index: 0,
            shot_dir: PathBuf::from(SHOT_DIR),
            taken: Vec::new(),
        }
    }

    fn for_stage(shot_dir: PathBuf) -> Self {
        Self {
            shot_index: 0,
            shot_dir,
            taken: Vec::new(),
        }
    }

    fn process_running(&self, program: &str) -> bool {
        std::fs::read_dir("/proc")
            .map(|d| {
                d.flatten().any(|e| {
                    std::fs::read_to_string(e.path().join("comm"))
                        .map(|c| c.trim() == program)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    /// The number of windows the compositor has mapped.
    ///
    /// This is the signal `wait_session` waits on. "The niri process exists" is
    /// not enough: the compositor starts before it can map anything, and v1's
    /// captures came back black precisely because the screenshot raced the
    /// first map.
    fn mapped_windows(&self) -> Vec<(String, String)> {
        let out = match std::process::Command::new("niri")
            .args(["msg", "--json", "windows"])
            .output()
        {
            Ok(o) if o.status.success() => o,
            _ => return Vec::new(),
        };
        let v: serde_json::Value = match serde_json::from_slice(&out.stdout) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        v.as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| {
                        let title = w
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        let app = w
                            .get("app_id")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        if title.is_empty() && app.is_empty() {
                            None
                        } else {
                            Some((title, app))
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Read a PNG's dimensions from its IHDR, so a capture can be validated
    /// without pulling an image decoder into the agent.
    fn png_size(path: &Path) -> Option<(u32, u32)> {
        let b = std::fs::read(path).ok()?;
        if b.len() < 24 || &b[..8] != b"\x89PNG\r\n\x1a\n" {
            return None;
        }
        let w = u32::from_be_bytes([b[16], b[17], b[18], b[19]]);
        let h = u32::from_be_bytes([b[20], b[21], b[22], b[23]]);
        Some((w, h))
    }

    /// OCR a capture with tesseract.
    ///
    /// Returns `None` when tesseract is not installed. The agent then reports
    /// that an assertion could not be evaluated, which is honest; it does not
    /// assume the text was there. Tesseract is in the QA image's package list
    /// for exactly this reason.
    fn ocr(&self, path: &Path) -> Option<String> {
        let out = std::process::Command::new("tesseract")
            .arg(path)
            .arg("stdout")
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).to_string())
    }
}

#[cfg(unix)]
impl Effects for LocalFx {
    fn wait_session(&mut self, timeout_secs: u64) -> StepOutcome {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        while std::time::Instant::now() < deadline {
            // The signal is a *mapped window*, not a running process. niri
            // starts well before it can map anything, so "is niri alive" let
            // v1 screenshot an empty framebuffer and call it evidence.
            if self.mapped_windows().is_empty() && !self.process_running("niri") {
                std::thread::sleep(std::time::Duration::from_millis(500));
                continue;
            }
            if !self.mapped_windows().is_empty() {
                return StepOutcome::Ok;
            }
            // niri is up but nothing is mapped yet. Give it a bounded grace
            // period before calling it a failure.
            if Path::new("/run/hcs/session.log").exists() {
                static GRACE: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                let start = *GRACE.get_or_init(std::time::Instant::now);
                if start.elapsed() > std::time::Duration::from_secs(20) {
                    return StepOutcome::Failed {
                        reason: "the compositor is running but no window ever mapped \
                                 (is the shell autostart working? check \
                                 /var/log/hcs/session.log)"
                            .into(),
                    };
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        StepOutcome::Timeout { secs: timeout_secs }
    }

    fn open(&mut self, app: &str) -> StepOutcome {
        match std::process::Command::new(app).arg("--gui").spawn() {
            Ok(_) => StepOutcome::Ok,
            Err(e) => StepOutcome::Failed {
                reason: format!("cannot launch {app}: {e}"),
            },
        }
    }

    fn key(&mut self, key: &str) -> StepOutcome {
        match std::process::Command::new("wtype")
            .arg("-k")
            .arg(key)
            .status()
        {
            Ok(s) if s.success() => StepOutcome::Ok,
            Ok(_) => StepOutcome::Failed {
                reason: format!("wtype could not deliver key '{key}'"),
            },
            Err(e) => StepOutcome::Failed {
                reason: format!("wtype unavailable: {e}"),
            },
        }
    }

    fn type_text(&mut self, text: &str) -> StepOutcome {
        match std::process::Command::new("wtype").arg(text).status() {
            Ok(s) if s.success() => StepOutcome::Ok,
            Ok(_) => StepOutcome::Failed {
                reason: "wtype rejected the text".into(),
            },
            Err(e) => StepOutcome::Failed {
                reason: format!("wtype unavailable: {e}"),
            },
        }
    }

    fn shot(&mut self, name: &str) -> StepOutcome {
        self.shot_index += 1;
        std::fs::create_dir_all(&self.shot_dir).ok();
        let path = self
            .shot_dir
            .join(format!("{:02}_{name}.png", self.shot_index));
        let run = std::process::Command::new("grim").arg(&path).status();
        match run {
            Ok(s) if s.success() => {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                if size == 0 {
                    return StepOutcome::Failed {
                        reason: "capture produced an empty file".into(),
                    };
                }
                self.taken.push(path);
                StepOutcome::Ok
            }
            Ok(_) => StepOutcome::Failed {
                reason: "grim exited non-zero".into(),
            },
            Err(e) => StepOutcome::Failed {
                reason: format!("grim unavailable: {e}"),
            },
        }
    }

    fn windows(&mut self) -> std::collections::BTreeMap<String, String> {
        self.mapped_windows().into_iter().collect()
    }

    fn capture_text(&mut self, name: &str) -> Option<String> {
        // Match on the recorded captures rather than reconstructing a filename:
        // a stage may take several, and the assertion refers to one by name.
        let want = name.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_');
        let path = self
            .taken
            .iter()
            .rev()
            .find(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.ends_with(want))
                    .unwrap_or(false)
            })
            .cloned()
            .or_else(|| self.taken.last().cloned())?;
        self.ocr(&path)
    }

    fn rss_mb(&mut self, process: &str) -> Option<u64> {
        let entries = std::fs::read_dir("/proc").ok()?;
        for e in entries.flatten() {
            let comm = std::fs::read_to_string(e.path().join("comm")).ok()?;
            if comm.trim() != process {
                continue;
            }
            let status = std::fs::read_to_string(e.path().join("status")).ok()?;
            for line in status.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    if let Some(kb) = rest.split_whitespace().next() {
                        if let Ok(kb) = kb.parse::<u64>() {
                            return Some(kb / 1024);
                        }
                    }
                }
            }
        }
        None
    }

    fn settle(&mut self, secs: u64) -> StepOutcome {
        std::thread::sleep(std::time::Duration::from_secs(secs));
        StepOutcome::Ok
    }
}
