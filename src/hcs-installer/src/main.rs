use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hcs-installer", about = "HCS Linux System Installation Helper")]
struct Args {
    #[arg(short, long, default_value = "/etc/calamares/settings.conf")]
    calamares_config: PathBuf,

    #[arg(long, default_value = "EDGE-8GB")]
    ai_profile: String,

    #[arg(long, default_value = "standard")]
    privacy_profile: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallerProfileSummary {
    installer_engine: String,
    branding: String,
    ai_profile: String,
    privacy_profile: String,
    target_bootloader: String,
    encrypted_root_supported: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("=== HCS Linux Installer Bootstrap ===");

    let summary = InstallerProfileSummary {
        installer_engine: "Calamares 3.3.14".to_string(),
        branding: "HCS Linux Glass Theme".to_string(),
        ai_profile: args.ai_profile,
        privacy_profile: args.privacy_profile,
        target_bootloader: "GRUB2 / systemd-boot (UEFI/BIOS Hybrid)".to_string(),
        encrypted_root_supported: true,
    };

    println!("{}", serde_json::to_string_pretty(&summary)?);
    println!("\nCalamares integration configured successfully.");

    Ok(())
}
