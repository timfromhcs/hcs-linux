//! `hcs-recall` — search the local timeline.
//!
//! Refuses to run in an amnesic session or without an explicit opt-in, because
//! "silently records nothing" would be indistinguishable from "records nothing
//! useful" and would leave the user believing they had a searchable history.

use anyhow::{bail, Result};
use hcs_recall::{allowed_in_session, search, space_allows, Decision, Snapshot};
use std::path::PathBuf;

const EXCLUDED_APPS: [&str; 6] = [
    "hcs-vault",
    "gnome-passwords",
    "keepassxc",
    "seahorse",
    "bitwarden",
    "1password",
];

fn store_dir() -> PathBuf {
    std::env::var("HCS_RECALL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/hcs/recall"))
}

fn load() -> Vec<Snapshot> {
    let index = store_dir().join("index.jsonl");
    let Ok(txt) = std::fs::read_to_string(index) else {
        return Vec::new();
    };
    txt.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Snapshot>(l).ok())
        .collect()
}

/// Keep the process alive briefly after doing its work, so an external sampler
/// can measure real RSS instead of racing a process that exits immediately.
/// Used by `scripts/gui_ram_audit.py`; harmless everywhere else.
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
    let amnesic = std::env::var("HCS_AMNESIC").ok().as_deref() == Some("1");
    let opted_in = std::env::var("HCS_RECALL_ENABLED").ok().as_deref() == Some("1");

    if !allowed_in_session(amnesic, opted_in) {
        bail!(
            "HCS Recall is not available in this session.\n\
             It requires an explicit opt-in (HCS_RECALL_ENABLED=1) and refuses to run in an \
             amnesic session, where it could record nothing and appear to work."
        );
    }

    let free_gb: u64 = std::env::var("HCS_FREE_GB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    if let Decision::Skip(reason) = space_allows(free_gb) {
        println!("capture paused: {reason}");
    }

    let snapshots = load();

    match args.first().map(String::as_str) {
        Some("search") => {
            let q = args[1..].join(" ");
            let hits = search(&snapshots, &q, 25);
            if json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else if hits.is_empty() {
                println!("Nothing in the timeline matches '{q}'.");
            } else {
                for h in &hits {
                    println!("  {:.2}  {}  {}", h.score, h.app, h.excerpt);
                }
            }
            Ok(())
        }
        Some("status") => {
            let total: u64 = snapshots.iter().map(|s| s.size_bytes).sum();
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "store": store_dir(),
                        "snapshots": snapshots.len(),
                        "bytes": total,
                        "excluded_apps": EXCLUDED_APPS,
                        "free_space_floor_gb": hcs_recall::FREE_SPACE_FLOOR_GB,
                        "network": "none — the index never leaves this machine",
                    })
                );
            } else {
                println!(
                    "Timeline:         {} snapshots, {} KB",
                    snapshots.len(),
                    total / 1024
                );
                println!("Store:            {}", store_dir().display());
                println!("Free-space floor: {} GB", hcs_recall::FREE_SPACE_FLOOR_GB);
                println!("Excluded apps:   {}", EXCLUDED_APPS.join(", "));
                println!("Network:         none — the index never leaves this machine");
            }
            Ok(())
        }
        _ => {
            eprintln!(
                "hcs-recall — local timeline search\n\n\
                 USAGE:\n    hcs-recall search <query>\n    hcs-recall status\n\n\
                 Requires HCS_RECALL_ENABLED=1. Never runs in an amnesic session."
            );
            Ok(())
        }
    }
}
