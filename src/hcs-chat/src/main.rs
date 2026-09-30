use clap::Parser;
use hcs_chat::gui;
use hcs_memory::MemoryEngine;
use hcs_modeld::{ModelDaemon, ModelEntry};
use hcs_security::PrivacyMode;
use hcs_settings::HardwareProfile;
use hcs_ui::ThemePreset;
use hcsd::HcsBrain;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hcs-chat", about = "HCS Linux Interactive AI Chat", version)]
struct Args {
    #[arg(short, long)]
    prompt: Option<String>,

    #[arg(long, default_value = "default")]
    project: String,

    /// Launch the Neural Glass GUI instead of the terminal REPL
    #[arg(long)]
    gui: bool,

    /// Theme preset for the GUI: obsidian (default), titanium, stealth
    #[arg(long, default_value = "obsidian")]
    theme: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mem = MemoryEngine::open_in_memory()?;
    let mut modeld = ModelDaemon::new(PathBuf::from("vendor/models"), 1);
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

    let hw = HardwareProfile::detect();
    let brain = HcsBrain::new(mem, modeld, hw, PrivacyMode::Standard);

    if args.gui {
        let theme = ThemePreset::from_str_opt(&args.theme).unwrap_or(ThemePreset::Obsidian);
        let ctx = gui::ChatContext::offline(&args.project);
        gui::run(&ctx, theme)?;
        return Ok(());
    }

    if let Some(user_prompt) = args.prompt {
        println!("User: {}", user_prompt);
        let result = brain
            .execute_task(&user_prompt, Some(&args.project))
            .await?;
        println!("\nHCS Brain [{}]:\n{}", result.model_used, result.response);
        println!(
            "\n[Task ID: {} | Verification: {}%]",
            result.task_id, result.scores.verification_quality
        );
        return Ok(());
    }

    println!("==================================================");
    println!("        Welcome to HCS Linux AI Assistant         ");
    println!(" Local-First • Privacy-Preserving • CPU-Optimized ");
    println!(" Type 'exit' or 'quit' to end session.             ");
    println!("==================================================\n");

    loop {
        print!("hcs> ");
        io::stdout().flush()?;
        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }
        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
            break;
        }
        if trimmed.is_empty() {
            continue;
        }

        let result = brain.execute_task(trimmed, Some(&args.project)).await?;
        println!("\n{}", result.response);
        println!(
            "[Model: {} | Score: {}]\n",
            result.model_used, result.scores.overall
        );
    }

    Ok(())
}
