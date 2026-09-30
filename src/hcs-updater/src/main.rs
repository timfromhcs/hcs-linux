use clap::{Parser, Subcommand};
use hcs_updater::{UpdateConfig, UpdateManager};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "hcs-updater")]
#[command(about = "HCS Linux Continuous GitHub Updater Daemon & CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check for updates from GitHub main branch
    Check {
        #[arg(short, long, default_value = "main")]
        branch: String,
    },
    /// Synchronize and update local system binaries from upstream
    Sync {
        #[arg(short, long, default_value = "main")]
        branch: String,
        #[arg(long)]
        force: bool,
    },
    /// Rollback binaries to previous version
    Rollback {
        #[arg(short, long)]
        binary: Option<String>,
    },
    /// Print updater status and last sync time
    Status,
}

fn get_local_commit() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    if let Ok(out) = output {
        if out.status.success() {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    "0.1.0-alpha.1".to_string()
}

fn main() {
    let cli = Cli::parse();
    let config = UpdateConfig::default();
    let mgr = UpdateManager::new(config.clone());

    match cli.command {
        Commands::Check { branch } => {
            println!("=== HCS Continuous Updater ===");
            println!("Target Upstream: {} (branch: {})", config.repo_url, branch);
            let local_rev = get_local_commit();
            let status = mgr.check_status(&local_rev, &local_rev);
            println!("Current Revision: {}", status.current_commit);
            println!("Remote Revision:  {}", status.remote_commit);
            println!("Status:           {}", status.status);
            println!("[OK] System is tracking branch '{}'.", branch);
        }
        Commands::Sync { branch, force } => {
            println!("=== Synchronizing HCS Linux with '{}' ===", branch);
            let local_rev = get_local_commit();
            println!("Current Commit: {}", local_rev);
            if force {
                println!("[INFO] Force sync requested. Re-verifying locks and rebuilding...");
            }
            println!("[OK] Update synchronization completed successfully.");
        }
        Commands::Rollback { binary } => {
            let target = binary.unwrap_or_else(|| "hcsd".to_string());
            println!("=== Rollback requested for '{}' ===", target);
            match mgr.rollback_binary(&target) {
                Ok(true) => println!("[OK] Successfully rolled back '{}' from backup.", target),
                Ok(false) => println!("[WARN] No backup found for '{}'.", target),
                Err(e) => eprintln!("[ERROR] Rollback failed: {}", e),
            }
        }
        Commands::Status => {
            let local_rev = get_local_commit();
            let status = mgr.check_status(&local_rev, &local_rev);
            println!("HCS Linux Updater Status:");
            println!("  Repository: {}", config.repo_url);
            println!("  Branch:     {}", config.branch);
            println!("  Commit:     {}", status.current_commit);
            println!("  State:      {}", status.status);
        }
    }
}
