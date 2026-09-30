use clap::Parser;
use hcs_memory::MemoryEngine;
use hcs_modeld::{ModelDaemon, ModelEntry};
use hcs_security::PrivacyMode;
use hcs_settings::HardwareProfile;
use hcsd::HcsBrain;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(
    name = "hcsd",
    version = "0.1.0-alpha.1",
    about = "HCS Linux Core Brain Daemon"
)]
struct Args {
    #[arg(short, long, default_value = "config/models/registry.yaml")]
    registry: PathBuf,

    #[arg(short, long, default_value = "vendor/models")]
    models_dir: PathBuf,

    #[arg(long, default_value = "hcs_memory.sqlite3")]
    db_path: PathBuf,

    #[arg(long)]
    private_mode: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    info!("Starting HCS Linux Brain Daemon (hcsd) v0.1.0-alpha.1...");

    let hw = HardwareProfile::detect();
    info!(
        "Detected hardware: {} cores, architecture: {}, profile: {}",
        hw.cpu_cores, hw.architecture, hw.recommended_profile
    );

    let memory = MemoryEngine::open(&args.db_path)
        .unwrap_or_else(|_| MemoryEngine::open_in_memory().expect("In-memory database fallback"));
    info!("Cognitive memory engine initialized at: {:?}", args.db_path);

    let mut modeld = ModelDaemon::new(args.models_dir.clone(), 1);
    modeld.register_model(ModelEntry {
        id: "hcs-assistant".to_string(),
        name: "Qwen3-1.7B-Assistant".to_string(),
        repo: "Qwen/Qwen3-1.7B".to_string(),
        filename: "qwen3-1.7b-q4_k_m.gguf".to_string(),
        revision: "a1c8f3e2d9b40716".to_string(),
        sha256: "c5d4e3f2a1b0987654321fedcba0987654321fedcba0987654321fedcba09876".to_string(),
        parameters: "1.7B".to_string(),
        quantization: "Q4_K_M".to_string(),
        role: "assistant".to_string(),
        runtime_tier: "interactive".to_string(),
        max_context: 8192,
        expected_ram_mb: 1450,
        license: "Apache-2.0".to_string(),
    });

    let privacy = if args.private_mode {
        PrivacyMode::PrivateTor
    } else {
        PrivacyMode::Standard
    };

    let brain = HcsBrain::new(memory, modeld, hw, privacy);
    info!("HCS Brain initialized successfully. Ready for incoming requests.");

    // Demo task evaluation
    let res = brain
        .execute_task("System health check and diagnostic", None)
        .await?;
    info!(
        "Initial self-test task executed: {} (Score: {})",
        res.task_id, res.scores.overall
    );

    let report = brain.generate_daily_report(1, 4150);
    info!("{}", report.summary_message);

    Ok(())
}
