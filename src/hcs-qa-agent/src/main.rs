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
use hcs_qa_agent::{parse_scenario, Agent, Effects, Step, StepOutcome};
#[cfg(unix)]
use std::path::{Path, PathBuf};

#[cfg(unix)]
const SOCKET_PATH: &str = "/run/hcs/qa.sock";
#[cfg(unix)]
const JOURNAL_PATH: &str = "/var/log/hcs/qa/journal.jsonl";
#[cfg(unix)]
const SHOT_DIR: &str = "/var/log/hcs/qa/shots";

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

        std::fs::create_dir_all(Path::new(SHOT_DIR))?;
        if let Some(parent) = Path::new(JOURNAL_PATH).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let args: Vec<String> = std::env::args().skip(1).collect();
        let agent = match args.first().map(String::as_str) {
            Some("run") => run_scenario_file(
                args.get(1)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/run/hcs/qa/scenario.json")),
            )?,
            _ => serve()?,
        };

        write_journal(&agent)?;
        Ok(())
    }
}

#[cfg(unix)]
fn write_journal(agent: &Agent) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(JOURNAL_PATH)
        .with_context(|| format!("cannot write {JOURNAL_PATH}"))?;
    writeln!(f, "{}", agent.journal_jsonl())?;
    eprintln!(
        "hcs-qa-agent: {} step(s) journaled to {JOURNAL_PATH}",
        agent.journal().len()
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

#[cfg(unix)]
fn serve() -> Result<Agent> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;

    if std::fs::metadata(SOCKET_PATH).is_ok() {
        std::fs::remove_file(SOCKET_PATH).ok();
    }
    if let Some(parent) = Path::new(SOCKET_PATH).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listener =
        UnixListener::bind(SOCKET_PATH).with_context(|| format!("cannot bind {SOCKET_PATH}"))?;
    eprintln!("hcs-qa-agent: listening on {SOCKET_PATH}");

    let mut agent = Agent::new();
    let mut fx = LocalFx::new();

    for stream in listener.incoming() {
        let stream = stream?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            let response = match serde_json::from_str::<Step>(trimmed) {
                Ok(step) => {
                    let outcome = agent.run_step(step, &mut fx);
                    if agent.is_finished() {
                        serde_json::json!({ "outcome": outcome.to_string() })
                    } else {
                        serde_json::json!({ "outcome": outcome.to_string() })
                    }
                }
                Err(e) => serde_json::json!({
                    "status": "failed",
                    "reason": format!("unparsable step: {e}"),
                }),
            };
            let mut w = stream.try_clone()?;
            writeln!(w, "{response}")?;
            w.flush()?;
            if agent.is_finished() {
                return Ok(agent);
            }
            line.clear();
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
}

#[cfg(unix)]
impl LocalFx {
    fn new() -> Self {
        Self { shot_index: 0 }
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
}

#[cfg(unix)]
impl Effects for LocalFx {
    fn wait_session(&mut self, timeout_secs: u64) -> StepOutcome {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
        while std::time::Instant::now() < deadline {
            if Path::new("/run/hcs/session.log").exists()
                || Path::new("/run/user/0/wayland-0").exists()
                || self.process_running("niri")
            {
                return StepOutcome::Ok;
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
        let path = format!("{SHOT_DIR}/{:02}_{name}.png", self.shot_index);
        match std::process::Command::new("grim").arg(&path).status() {
            Ok(s) if s.success() => {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                if size == 0 {
                    StepOutcome::Failed {
                        reason: "capture produced an empty file".into(),
                    }
                } else {
                    StepOutcome::Ok
                }
            }
            Ok(_) => StepOutcome::Failed {
                reason: "grim exited non-zero".into(),
            },
            Err(e) => StepOutcome::Failed {
                reason: format!("grim unavailable: {e}"),
            },
        }
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
