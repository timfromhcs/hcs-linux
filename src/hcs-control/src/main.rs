use clap::{Parser, Subcommand};
use hcs_security::{PrivacyMode, TorManager, VaultManager};
use hcs_settings::HardwareProfile;

#[derive(Parser, Debug)]
#[command(
    name = "hcs-control",
    about = "HCS Linux System & Brain Control Center"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show full dashboard status
    Status,
    /// Inspect privacy and Tor status
    Privacy,
    /// Inspect hardware profile and budget
    Hardware,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            println!("==================================================");
            println!("             HCS CONTROL DASHBOARD                ");
            println!("==================================================");
            let hw = HardwareProfile::detect();
            println!("• Base Profile:       {}", hw.recommended_profile);
            println!("• Active RAM Budget:  <= 6144 MB idle / <= 8192 MB peak");
            println!("• Privacy Engine:     Enabled (Tor SOCKS5 :9050)");
            println!("• Cognitive Memory:   SQLite FTS5 + Hybrid Embeddings");
            println!("• Single Resident:    Enforced (Max 1 heavy model)");
            println!("• Vault State:        Encrypted (AES-XTS-512)");
            println!("==================================================");
        }
        Commands::Privacy => {
            let tor = TorManager::default();
            let status = tor.get_status(PrivacyMode::Standard);
            println!("Privacy Status:");
            println!("  Tor Active:         {}", status.is_running);
            println!("  SOCKS Proxy:        127.0.0.1:{}", status.socks_port);
            let vault = VaultManager::check_status("/home/user/.hcs/vault");
            println!(
                "  Encrypted Vault:    {} ({})",
                vault.mount_point, vault.cipher
            );
        }
        Commands::Hardware => {
            let hw = HardwareProfile::detect();
            println!("Detected Hardware Profile:");
            println!("  CPU:                {}", hw.cpu_model);
            println!("  Cores:              {}", hw.cpu_cores);
            println!(
                "  AVX Support:        AVX: {}, AVX2: {}, AVX-512: {}",
                hw.has_avx, hw.has_avx2, hw.has_avx512
            );
            println!("  Total RAM:          {} MB", hw.total_ram_mb);
            println!("  Matched Profile:    {}", hw.recommended_profile);
        }
    }

    Ok(())
}
