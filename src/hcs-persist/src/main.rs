//! `hcs-persist` — inspect and configure session persistence.

use anyhow::Result;
use hcs_persist::{kernel_parameters, passphrase_words, FeatureState, Persistence, VolumeState};

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

    let mut p = Persistence::builtin();
    // The on-disk state, when present, wins over the built-in defaults.
    if let Ok(path) = std::env::var("HCS_PERSIST_STATE") {
        if let Ok(txt) = std::fs::read_to_string(&path) {
            if let Ok(stored) = serde_json::from_str::<Persistence>(&txt) {
                p = stored;
            }
        }
    }
    p.volume = match std::env::var("HCS_PERSIST_UNLOCKED").ok().as_deref() {
        Some("1") => VolumeState::Unlocked,
        Some(_) => VolumeState::Locked,
        None => VolumeState::Absent,
    };

    let outcome: Result<()> = match args.first().map(String::as_str) {
        Some("status") => {
            let params = kernel_parameters(&p, false, false, false);
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "volume": p.volume,
                        "amnesic": matches!(p.volume, VolumeState::Absent),
                        "memory_poisoning": p.memory_poisoning,
                        "swap_disabled": p.swap_disabled,
                        "active": p.to_mount().iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
                        "kernel_params": params,
                    })
                );
            } else {
                println!("Persistent storage: {:?}", p.volume);
                println!(
                    "Amnesic session:  {}",
                    matches!(p.volume, VolumeState::Absent)
                );
                println!("Memory poisoning: {}", p.memory_poisoning);
                println!("Swap disabled:    {}", p.swap_disabled);
                println!("Kernel parameters applied:");
                for k in &params {
                    println!("  {k}");
                }
                let active = p.to_mount();
                println!("Active features:  {}", active.len());
                for f in active {
                    println!("  [{}] {}", f.state.as_str(), f.id);
                }
            }
            Ok(())
        }
        Some("features") => {
            if json {
                println!("{}", serde_json::to_string_pretty(&p.features)?);
            } else {
                for f in &p.features {
                    println!("  {:<12} {:<10} {}", f.id, f.state.as_str(), f.name);
                }
            }
            Ok(())
        }
        Some("activate") => {
            let id = args.get(1).cloned().unwrap_or_default();
            p.activate(&id, matches!(p.volume, VolumeState::Unlocked))?;
            println!("[OK] feature '{id}' is now {}", FeatureState::Active);
            Ok(())
        }
        Some("deactivate") => {
            let id = args.get(1).cloned().unwrap_or_default();
            p.deactivate(&id)?;
            println!(
                "[OK] feature '{id}' is now {} (data kept)",
                FeatureState::Enabled
            );
            Ok(())
        }
        Some("mask") => {
            let id = args.get(1).cloned().unwrap_or_default();
            p.mask(&id)?;
            println!("[OK] feature '{id}' is now {}", FeatureState::Masked);
            Ok(())
        }
        Some("passphrase") => {
            let n = args
                .get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(6);
            let words = passphrase_words(n);
            if json {
                println!("{}", serde_json::json!({ "words": words }));
            } else {
                println!("{}", words.join("-"));
                println!();
                println!("Six words is enough to resist guessing if you actually");
                println!("memorise them; write them down before you rely on them.");
            }
            Ok(())
        }
        _ => {
            eprintln!(
                "hcs-persist — amnesic sessions and opt-in persistence\n\n\
                 USAGE:\n    hcs-persist status            Volume, safeguards and kernel parameters\n    \
                 hcs-persist features          List persistable features and their state\n    \
                 hcs-persist activate <id>     Mount a feature (needs an unlocked volume)\n    \
                 hcs-persist deactivate <id>   Unmount a feature, keeping its data\n    \
                 hcs-persist mask <id>         Hide a feature without deleting its data\n    \
                 hcs-persist passphrase [5-7]  Suggest a memorable passphrase\n\n\
                 With no persistent volume configured the session is fully amnesic:\n\
                 nopersistence, noswap and init_on_free=1 are applied."
            );
            Ok(())
        }
    };

    hold_for_measurement(&args);
    outcome
}
