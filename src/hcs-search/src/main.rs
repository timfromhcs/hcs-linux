use clap::Parser;
use hcs_memory::MemoryEngine;
use hcs_ui::ThemePreset;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "hcs-search", about = "HCS Linux Fast Hybrid Search", version)]
struct Args {
    /// Launch the Neural Glass Spotlight GUI
    #[arg(long)]
    gui: bool,

    /// Theme preset for the GUI: obsidian (default), titanium, stealth
    #[arg(long, default_value = "obsidian")]
    theme: String,

    #[arg(short, long)]
    query: Option<String>,

    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    #[arg(long, default_value = "hcs_memory.sqlite3")]
    db_path: PathBuf,

    #[arg(short, long, default_value_t = 10)]
    limit: usize,
}

fn search_filesystem_exact(root: &Path, query: &str, limit: usize) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    let query_lower = query.to_lowercase();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_lowercase();
            if file_name.contains(&query_lower) {
                matches.push(entry.path());
                if matches.len() >= limit {
                    break;
                }
            }
        }
    }
    matches
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.gui {
        let theme = ThemePreset::from_str_opt(&args.theme).unwrap_or(ThemePreset::Obsidian);
        hcs_search::gui::run(theme)?;
        return Ok(());
    }

    let Some(query) = args.query else {
        println!("run with --gui for the Spotlight GUI, or pass -q/--query for CLI search.");
        return Ok(());
    };

    println!("=== HCS Unified Search ===");
    println!("Query: '{}'\n", query);

    // Level 1: Fast exact filesystem match
    let fs_matches = search_filesystem_exact(&args.path, &query, args.limit);
    if !fs_matches.is_empty() {
        println!("[Filesystem Matches]");
        for path in fs_matches {
            println!("  -> {}", path.display());
        }
        println!();
    }

    // Level 2: FTS5 Cognitive Memory Search
    if let Ok(mem) = MemoryEngine::open(&args.db_path) {
        if let Ok(results) = mem.hybrid_search(&query, None, args.limit) {
            if !results.is_empty() {
                println!("[Cognitive Memory Matches]");
                for m in results {
                    println!(
                        "  [{:.2}] ({:?}) {}",
                        m.total_score, m.record.memory_class, m.record.content
                    );
                }
            }
        }
    }

    Ok(())
}
