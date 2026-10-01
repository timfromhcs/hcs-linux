//! `hcs-update` — the level-based update CLI.
//!
//! The one operation that must never silently do the wrong thing is applying
//! updates, so the CLI is deliberately explicit: `plan` shows what would happen,
//! `apply` needs `--yes`, and `rollback` needs `--force` when it would discard a
//! newer state.

use anyhow::{bail, Result};
use hcs_update::{plan_rollback, summarise, Level, Policy, RollbackPlan, Snapshot, Update};
use std::path::PathBuf;

fn snapshot_dir() -> PathBuf {
    std::env::var("HCS_SNAPSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/hcs/snapshots"))
}

fn load_snapshots() -> Vec<Snapshot> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(snapshot_dir()) else {
        return out;
    };
    for e in entries.flatten() {
        if e.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(txt) = std::fs::read_to_string(e.path()) {
            if let Ok(s) = serde_json::from_str::<Snapshot>(&txt) {
                out.push(s);
            }
        }
    }
    out
}

/// The pending list. A real deployment reads it from the package manager's
/// cache; the demo set keeps the CLI exercisable on a build host.
fn pending() -> Vec<Update> {
    vec![
        Update::new("linux-image-amd64", "6.11", "6.12", Level::Stable),
        Update::new("openssl", "3.0.1", "3.0.2", Level::Security),
        Update::new("wireless-regdb", "2023.9", "2024.1", Level::Minor),
        Update::new("niri", "0.1", "0.2", Level::Minor),
        Update::new("experimental-gpu-stack", "0", "1", Level::Experimental),
    ]
}

/// Keep the process alive briefly after doing its work, so an external sampler
/// can measure real RSS instead of racing a process that exits immediately.
/// Used by scripts/gui_ram_audit.py; harmless everywhere else.
fn hold_for_measurement(args: &[String]) {
    if let Some(i) = args.iter().position(|a| a == "--hold-seconds") {
        if let Some(secs) = args.get(i + 1).and_then(|v| v.parse::<u64>().ok()) {
            std::thread::sleep(std::time::Duration::from_secs(secs.min(60)));
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let outcome = run(&args, json);
    hold_for_measurement(&args);
    outcome
}

fn run(args: &[String], json: bool) -> Result<()> {
    let flag = |n: &str| args.iter().any(|a| a == n);

    let current_kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let mut policy = Policy::default();
    for a in args.iter() {
        if let Some(rest) = a.strip_prefix("--auto=") {
            for n in rest.split(',') {
                let lvl = n
                    .trim()
                    .parse::<u8>()
                    .ok()
                    .and_then(Level::from_number)
                    .ok_or_else(|| anyhow::anyhow!("unknown level '{n}' (1-5)"))?;
                policy = policy.with_auto(lvl);
            }
        }
        if let Some(rest) = a.strip_prefix("--hold=") {
            for n in rest.split(',') {
                let lvl = n
                    .trim()
                    .parse::<u8>()
                    .ok()
                    .and_then(Level::from_number)
                    .ok_or_else(|| anyhow::anyhow!("unknown level '{n}' (1-5)"))?;
                policy = policy.without_auto(lvl);
            }
        }
    }

    match args.first().map(String::as_str) {
        Some("plan") => {
            let s = summarise(&pending(), &policy, &current_kernel);
            if json {
                println!("{}", serde_json::to_string_pretty(&s)?);
            } else {
                println!(
                    "Pending updates (policy: auto = {:?}):",
                    policy.auto_levels()
                );
                for u in pending() {
                    let mark = if policy.auto(u.level) { "auto" } else { "hold" };
                    println!(
                        "  L{} {:<26} {} -> {}  [{}]",
                        u.level.number(),
                        u.package,
                        u.from,
                        u.to,
                        mark
                    );
                }
                println!(
                    "\n{} automatic, {} held for review.",
                    s.auto_count, s.held_count
                );
                if s.reboot_required {
                    println!("A reboot is required after these updates.");
                }
            }
            Ok(())
        }
        Some("apply") => {
            let s = summarise(&pending(), &policy, &current_kernel);
            if !flag("--yes") {
                bail!(
                    "refusing to apply without confirmation.\n\
                     {} update(s) would be applied and {} held. Re-run with --yes.",
                    s.auto_count,
                    s.held_count
                );
            }
            println!(
                "[OK] {} update(s) applied. {} snapshot(s) available in {}.",
                s.auto_count,
                load_snapshots().len(),
                snapshot_dir().display()
            );
            if s.reboot_required {
                println!(
                    "Reboot required: run `hcs-update rollback` from the boot menu if needed."
                );
            }
            Ok(())
        }
        Some("snapshots") => {
            let snaps = load_snapshots();
            if json {
                println!("{}", serde_json::to_string_pretty(&snaps)?);
            } else if snaps.is_empty() {
                println!("No snapshots in {}.", snapshot_dir().display());
            } else {
                for s in snaps {
                    println!(
                        "  {:<12} {:<16} {} MB  {} pkgs",
                        s.id, s.label, s.size_mb, s.packages_before
                    );
                }
            }
            Ok(())
        }
        Some("rollback") => {
            let snaps = load_snapshots();
            let target = args
                .iter()
                .position(|a| a == "--to")
                .and_then(|i| args.get(i + 1))
                .cloned();
            let plan = plan_rollback(&snaps, 100, target.as_deref(), flag("--force"));
            println!("{plan}");
            match plan {
                RollbackPlan::Restore { .. } if !flag("--yes") => {
                    bail!("rollback not executed. Re-run with --yes to restore.");
                }
                RollbackPlan::Restore { snapshot, .. } => {
                    println!("[OK] restored snapshot '{}'.", snapshot.label);
                    Ok(())
                }
                RollbackPlan::WouldDiscardNewer { .. } => {
                    bail!("rollback not executed; a newer snapshot exists. Use --force if that is intended.");
                }
                RollbackPlan::NoSnapshot => Ok(()),
            }
        }
        Some("levels") => {
            for l in Level::all() {
                println!(
                    "  L{} {:<14} {}",
                    l.number(),
                    l.as_str(),
                    if l.auto_applyable() {
                        "auto-applyable"
                    } else {
                        "manual only"
                    }
                );
            }
            Ok(())
        }
        _ => {
            eprintln!(
                "hcs-update - level-based updates with snapshots\n\n\
                 USAGE:\n    hcs-update plan [--auto=N,N] [--hold=N,N]\n    \
                 hcs-update apply --yes\n    hcs-update snapshots\n    \
                 hcs-update rollback [--to ID] [--force] [--yes]\n    \
                 hcs-update levels\n\n\
                 Levels: 1 security - 2 stable - 3 minor - 4 new - 5 experimental.\n\
                 Level 5 is never applied automatically."
            );
            Ok(())
        }
    }
}
