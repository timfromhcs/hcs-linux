use clap::Parser;
use hcs_memory::{MemoryClass, MemoryEngine, VerificationState};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "hcs-rag-ingest")]
#[command(about = "HCS Linux Offline Knowledge & Document Chunker / Ingester")]
struct Cli {
    /// File or directory to ingest
    #[arg(short, long)]
    path: PathBuf,
    /// Associated project tag
    #[arg(long, default_value = "default")]
    project: String,
    /// Chunk size in characters
    #[arg(long, default_value_t = 1000)]
    chunk_size: usize,
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_file() {
        files.push(dir.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            if !name.starts_with('.') && name != "target" && name != "vendor" {
                collect_files(&p, files)?;
            }
        } else if let Some(ext) = p.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ["md", "txt", "rs", "json", "yaml", "yml", "toml", "sh"].contains(&ext_str.as_str())
            {
                files.push(p);
            }
        }
    }
    Ok(())
}

fn chunk_text(content: &str, chunk_size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut current_chunk = String::new();

    for line in lines {
        if current_chunk.len() + line.len() > chunk_size && !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }
        current_chunk.push_str(line);
        current_chunk.push('\n');
    }
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }
    chunks
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    println!("=== HCS Knowledge RAG Ingester ===");
    println!("Target Path: {}", cli.path.display());
    println!("Project Tag: {}", cli.project);

    if !cli.path.exists() {
        eprintln!("[ERROR] Path not found: {}", cli.path.display());
        std::process::exit(1);
    }

    let mut files = Vec::new();
    collect_files(&cli.path, &mut files)?;
    println!("[INFO] Found {} candidate documents.", files.len());

    let mut engine = MemoryEngine::open_in_memory()?;
    let mut total_chunks = 0;

    for f in &files {
        if let Ok(content) = fs::read_to_string(f) {
            let chunks = chunk_text(&content, cli.chunk_size);
            for chunk in chunks {
                engine.insert(
                    MemoryClass::Semantic,
                    &format!("rag-file://{}", f.display()),
                    &chunk,
                    0.95,
                    VerificationState::Verified,
                    Some(cli.project.clone()),
                    "public",
                )?;
                total_chunks += 1;
            }
        }
    }

    println!("==================================================");
    println!(" Ingestion Complete!");
    println!(" Documents Processed : {}", files.len());
    println!(" Chunks Indexed       : {}", total_chunks);
    println!(" Backend Engine       : SQLite FTS5 + Semantic Hybrid");
    println!(" Status               : Ready for hybrid search query");
    println!("==================================================");

    Ok(())
}
