use clap::{Parser, Subcommand};
use hcs_mcp::{JsonRpcRequest, McpServer};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hcs-mcp")]
#[command(about = "HCS Linux Native Model Context Protocol (MCP) Server & Hub")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Serve MCP tools over JSON-RPC 2.0 stdio transport
    Serve,
    /// List all registered local MCP tools
    List,
    /// Export standard MCP client configuration for Claude Desktop and Cursor
    ExportConfig {
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Directly invoke an MCP tool
    Call {
        #[arg(short, long)]
        tool: String,
        #[arg(short, long, default_value = "{}")]
        args: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let server = McpServer::default();

    match cli.command {
        Commands::Serve => {
            server.run_stdio_server().await?;
        }
        Commands::List => {
            println!("HCS Linux Native MCP Tools:");
            for t in server.list_tools() {
                println!("  - {:<18} : {}", t.name, t.description);
            }
        }
        Commands::ExportConfig { out } => {
            let target_path = out.unwrap_or_else(|| {
                dirs_fallback()
                    .join(".config")
                    .join("hcs")
                    .join("mcp_servers.json")
            });

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let cfg = json!({
                "mcpServers": {
                    "hcs-native": {
                        "command": "/usr/bin/hcs-mcp",
                        "args": ["serve"]
                    }
                }
            });

            fs::write(&target_path, serde_json::to_string_pretty(&cfg)?)?;
            println!(
                "[OK] MCP configuration exported to: {}",
                target_path.display()
            );
        }
        Commands::Call { tool, args } => {
            let parsed_args: serde_json::Value = serde_json::from_str(&args)?;
            let req = JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(json!(1)),
                method: "tools/call".to_string(),
                params: Some(json!({
                    "name": tool,
                    "arguments": parsed_args
                })),
            };
            let resp = server.handle_request(req).await;
            if let Some(result) = resp.result {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if let Some(err) = resp.error {
                eprintln!("[ERROR] MCP Call failed: {}", err.message);
            }
        }
    }

    Ok(())
}

fn dirs_fallback() -> PathBuf {
    #[cfg(unix)]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    #[cfg(windows)]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            return PathBuf::from(userprofile);
        }
    }
    PathBuf::from(".")
}
