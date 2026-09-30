//! `hcs-notes` — notes CLI. The GUI window is a thin layer over this; the
//! store logic lives in the library so it is testable without a display.

use anyhow::Result;
use hcs_notes::{render_markdown, search, Note};
use std::path::{Path, PathBuf};

fn notes_dir() -> PathBuf {
    std::env::var("HCS_NOTES_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("XDG_DATA_HOME")
                .map(|d| PathBuf::from(d).join("hcs/notes"))
                .or_else(|_| {
                    std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/share/hcs/notes"))
                })
        })
        .unwrap_or_else(|_| PathBuf::from("/var/lib/hcs/notes"))
}

fn load_all(dir: &Path) -> Vec<Note> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        // First heading, if present, is the title; otherwise the file stem is.
        let title = body
            .lines()
            .find_map(|l| l.strip_prefix("# ").map(str::trim))
            .unwrap_or(&id)
            .to_string();
        out.push(Note {
            id,
            title,
            body,
            modified,
        });
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.id.cmp(&b.id)));
    out
}

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
    let dir = notes_dir();
    let notes = load_all(&dir);

    match args.first().map(String::as_str) {
        Some("list") => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "dir": dir,
                        "count": notes.len(),
                        "notes": notes.iter().map(|n| serde_json::json!({
                            "id": n.id,
                            "title": n.title,
                            "modified": n.modified,
                        })).collect::<Vec<_>>(),
                    })
                );
            } else if notes.is_empty() {
                println!(
                    "No notes in {}. Create one with: hcs-notes new <title>",
                    dir.display()
                );
            } else {
                for n in &notes {
                    println!("  {:<20} {}", n.id, n.title);
                }
            }
        }
        Some("search") => {
            let q = args[1..].join(" ");
            let hits = search(&notes, &q, 20);
            if json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else {
                for h in &hits {
                    println!("  {:.2}  {}", h.score, h.snippet);
                }
            }
        }
        Some("render") => {
            let id = args.get(1).cloned().unwrap_or_default();
            let note = notes
                .iter()
                .find(|n| n.id == id)
                .ok_or_else(|| anyhow::anyhow!("no note with id '{id}'"))?;
            print!("{}", render_markdown(&note.body));
        }
        Some("new") => {
            let title = args[1..].join(" ");
            if title.is_empty() {
                anyhow::bail!("usage: hcs-notes new <title>");
            }
            std::fs::create_dir_all(&dir)?;
            let id = title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>();
            let path = dir.join(format!("{id}.md"));
            if path.exists() {
                anyhow::bail!("note '{id}' already exists");
            }
            std::fs::write(&path, format!("# {title}\n\n"))?;
            println!("{}", path.display());
        }
        _ => {
            eprintln!(
                "hcs-notes — Markdown notes\n\n\
                 USAGE:\n    hcs-notes list              List notes\n    \
                 hcs-notes search <query>     Search titles and bodies\n    \
                 hcs-notes render <id>        Print rendered Markdown\n    \
                 hcs-notes new <title>        Create a note\n\n\
                 All verbs accept --json."
            );
        }
    }
    hold_for_measurement(&args);
    Ok(())
}
