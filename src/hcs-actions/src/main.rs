//! `hcs-actions` — inspect and dispatch the HCS action registry.
//!
//! The CLI half of the omnibar's contract. Every subcommand supports `--json`
//! so an agent (or a script) can drive the desktop without screen-scraping.
//! See docs/V2_STABLE_RELEASE_MASTER_PLAN.md §4.5: one registry, three front
//! ends (omnibar, CLI, agent).

use anyhow::{Context, Result};
use hcs_actions::{builtin_registry, Category, QuickKeyResolution};
use std::process::Command;

fn main() -> Result<()> {
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");

    // Positional arguments, with flag *values* removed so `run app.chat --yes`
    // does not treat "--yes" as an id. Flag values are consumed explicitly below
    // where each verb knows its own options.
    let valued_flags = [
        "--theme",
        "--output",
        "--label",
        "--format",
        "--geometry",
        "--title",
    ];
    let mut positional: Vec<String> = Vec::new();
    let mut skip_next = false;
    for a in &args {
        if skip_next {
            skip_next = false;
            continue;
        }
        if a.starts_with("--") {
            if valued_flags.contains(&a.as_str()) {
                skip_next = true;
            }
            continue;
        }
        positional.push(a.clone());
    }

    let reg = builtin_registry();
    let _ = &reg;

    let (verb, rest) = match positional.split_first() {
        Some((v, r)) => (v.clone(), r.to_vec()),
        None => {
            usage();
            return Ok(());
        }
    };

    match verb.as_str() {
        "list" => {
            let category = rest.first().and_then(|c| parse_category(c));
            let items = reg.browse(category);
            if json {
                let out: Vec<_> = items
                    .iter()
                    .map(|a| serde_json::to_value(a).unwrap())
                    .collect();
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                for a in items {
                    let risk = if a.requires_confirmation {
                        " [CONFIRM]"
                    } else {
                        ""
                    };
                    let key = a
                        .quick_key
                        .as_ref()
                        .map(|k| format!(" ({k})"))
                        .unwrap_or_default();
                    println!("  {:<34}{:<9}{}{}", a.id, a.category.as_str(), key, risk);
                }
            }
            Ok(())
        }
        "search" => {
            let query = rest.join(" ");
            let hits = reg.search(&query, 20);
            if json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else if hits.is_empty() {
                println!("No actions match '{query}'.");
            } else {
                for h in hits {
                    println!("  {:.2}  {}", h.score, h.action.id);
                }
            }
            Ok(())
        }
        "quickkey" => {
            let key = rest.first().cloned().unwrap_or_default();
            match reg.resolve_quick_key(&key) {
                QuickKeyResolution::Resolved(id) => {
                    println!("{id}");
                    Ok(())
                }
                QuickKeyResolution::Ambiguous(ids) => {
                    anyhow::bail!("quick key '{}' is ambiguous: {:?}", key, ids)
                }
                QuickKeyResolution::NotFound => {
                    anyhow::bail!("no action bound to quick key '{key}'")
                }
            }
        }
        "describe" => {
            let id = rest.first().cloned().unwrap_or_default();
            let action = reg
                .get(&id)
                .with_context(|| format!("unknown action '{id}'"))?;
            if json {
                println!("{}", serde_json::to_string_pretty(action)?);
            } else {
                println!("id:          {}", action.id);
                println!("title:       {}", action.title);
                println!("category:    {}", action.category.as_str());
                println!("risk:        {:?}", action.risk);
                println!("confirm:     {}", action.requires_confirmation);
                println!(
                    "quick key:   {}",
                    action.quick_key.as_deref().unwrap_or("-")
                );
                println!("description: {}", action.description);
                println!(
                    "command:     {}",
                    action
                        .command
                        .as_ref()
                        .map(|c| c.join(" "))
                        .unwrap_or_else(|| "(none)".into())
                );
            }
            Ok(())
        }
        "run" => {
            let id = rest.first().cloned().unwrap_or_default();
            let confirmed = args.iter().any(|a| a == "--yes" || a == "--confirm");
            let dry = args.iter().any(|a| a == "--dry-run");
            let argv = reg.argv_for(&id, confirmed)?;

            if json || dry {
                println!(
                    "{}",
                    serde_json::json!({ "id": id, "argv": argv, "executed": !dry })
                );
                if dry {
                    return Ok(());
                }
            }

            let (program, args) = argv
                .split_first()
                .with_context(|| "action has an empty command")?;
            let status = Command::new(program).args(args).status();
            match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => anyhow::bail!("'{program}' exited with {s}"),
                Err(e) => Err(e).with_context(|| format!("failed to launch '{program}'")),
            }
        }
        "validate" => {
            let problems = reg.validate();
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "ok": problems.is_empty(), "problems": problems })
                );
            } else if problems.is_empty() {
                println!("[OK] action registry consistent ({} actions).", reg.len());
            } else {
                for p in &problems {
                    println!("  [PROBLEM] {p}");
                }
            }
            if problems.is_empty() {
                hold_for_measurement(&args);
                Ok(())
            } else {
                anyhow::bail!("action registry has {} problem(s)", problems.len())
            }
        }
        other => {
            anyhow::bail!(
                "unknown verb '{other}' (try: list, search, quickkey, describe, run, validate)"
            )
        }
    }
}

fn parse_category(s: &str) -> Option<Category> {
    match s.to_lowercase().as_str() {
        "app" | "apps" => Some(Category::App),
        "window" | "windows" => Some(Category::Window),
        "system" => Some(Category::System),
        "privacy" => Some(Category::Privacy),
        "ai" => Some(Category::Ai),
        "files" => Some(Category::Files),
        "settings" => Some(Category::Settings),
        "update" => Some(Category::Update),
        "help" => Some(Category::Help),
        _ => None,
    }
}

fn usage() {
    eprintln!(
        "hcs-actions — HCS action registry

USAGE:
    hcs-actions list [category]        List actions, optionally by category
    hcs-actions search <query>         Rank actions against a query
    hcs-actions quickkey <key>         Resolve a quick key to an action id
    hcs-actions describe <id>          Show one action in full
    hcs-actions run <id> [--yes]      Execute an action (privileged needs --yes)
    hcs-actions run <id> --dry-run     Print what would run
    hcs-actions validate              Check the registry for problems

All verbs accept --json."
    );
}
