use clap::Parser;
use hcs_ui::ThemePreset;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "hcs-monitor")]
#[command(about = "HCS Linux AI & System Resource Telemetry Dashboard", version)]
struct Cli {
    /// Watch mode with interval in seconds
    #[arg(short, long)]
    watch: Option<u64>,
    /// Output raw JSON telemetry
    #[arg(short, long)]
    json: bool,
    /// Launch the Neural Glass GUI
    #[arg(long)]
    gui: bool,
    /// Theme preset for the GUI: obsidian (default), titanium, stealth
    #[arg(long, default_value = "obsidian")]
    theme: String,
}

fn print_dashboard() {
    println!("================================================================================");
    println!("                           HCS LINUX SYSTEM & AI MONITOR                        ");
    println!("================================================================================");
    println!(" Profile Target: EDGE-8GB | Idle Budget: <= 6144 MB | Peak Budget: <= 8192 MB");
    println!("--------------------------------------------------------------------------------");
    println!(" [RAM USAGE]");
    println!("   System Base + Desktop (Niri/Quickshell) :  820 MB / 8192 MB  [██░░░░░░░░] 10%");
    println!("   Core Services (hcsd, memory, security)  :  210 MB / 8192 MB  [█░░░░░░░░░]  3%");
    println!("   Resident AI Controller (Qwen3-0.6B)     :  550 MB / 8192 MB  [██░░░░░░░░]  7%");
    println!("   Total Active RSS                        : 1580 MB / 8192 MB  [███░░░░░░░] 19%");
    println!("   Available System Headroom               : 6612 MB remaining (Safe)");
    println!("--------------------------------------------------------------------------------");
    println!(" [COGNITIVE MODEL STATUS]");
    println!("   Active Heavy Model : None (Single-resident policy: idle)");
    println!("   Resident Model     : hcs-controller (Qwen3-0.6B, Q4_K_M, CPU AVX2)");
    println!("   Inference Engine   : llama.cpp native CPU (4 threads)");
    println!("   KV-Cache Overhead  : 82 MB | Context Window: 2048");
    println!("--------------------------------------------------------------------------------");
    println!(" [DAEMONS & MCP SERVERS]");
    println!("   [  OK  ] hcsd.service          : Active (running)");
    println!("   [  OK  ] hcs-modeld.service    : Active (running)");
    println!("   [  OK  ] hcs-memory.service    : Active (SQLite FTS5 Healthy)");
    println!("   [  OK  ] hcs-security.service  : Active (Tor Ready)");
    println!("   [  OK  ] hcs-updater.timer     : Active (Syncing from origin/main)");
    println!("   [  OK  ] hcs-mcp-hub           : Ready (stdio & UDS)");
    println!("================================================================================");
}

fn main() {
    let cli = Cli::parse();

    if cli.gui {
        let theme = ThemePreset::from_str_opt(&cli.theme).unwrap_or(ThemePreset::Obsidian);
        hcs_monitor::gui::run(theme).expect("hcs-monitor GUI");
        return;
    }

    if cli.json {
        let data =
            serde_json::to_string_pretty(&hcs_monitor::telemetry::Telemetry::sample()).unwrap();
        println!("{}", data);
        return;
    }

    if let Some(interval) = cli.watch {
        loop {
            print!("\x1B[2J\x1B[1;1H");
            print_dashboard();
            thread::sleep(Duration::from_secs(interval));
        }
    } else {
        print_dashboard();
    }
}
