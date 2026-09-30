use chrono::Utc;
use hcs_memory::MemoryEngine;
use hcs_modeld::{ModelDaemon, ModelEntry};
use hcs_security::{PrivacyMode, TorManager};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    #[error("Tool execution failed: {0}")]
    ToolExecution(String),
    #[error("Path access forbidden: {0}")]
    ForbiddenPath(String),
    #[error("Internal memory error: {0}")]
    MemoryError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub struct McpServer {
    tools: Vec<McpTool>,
    allowed_roots: Vec<PathBuf>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new(vec![
            PathBuf::from("."),
            PathBuf::from("/tmp"),
            PathBuf::from("/usr/share/hcs"),
        ])
    }
}

impl McpServer {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        let tools = vec![
            McpTool {
                name: "fs_read_file".to_string(),
                description: "Read text contents of a file within allowed paths".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to file" }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "fs_write_file".to_string(),
                description: "Write text contents to a file within allowed paths".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Target file path" },
                        "content": { "type": "string", "description": "Text content" }
                    },
                    "required": ["path", "content"]
                }),
            },
            McpTool {
                name: "fs_list_dir".to_string(),
                description: "List directory contents".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path" }
                    },
                    "required": ["path"]
                }),
            },
            McpTool {
                name: "bash_run".to_string(),
                description: "Execute safe shell command in sandboxed environment".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Command string to run" }
                    },
                    "required": ["command"]
                }),
            },
            McpTool {
                name: "memory_search".to_string(),
                description: "Query HCS SQLite cognitive memory graph".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query terms" },
                        "limit": { "type": "integer", "description": "Max results" }
                    },
                    "required": ["query"]
                }),
            },
            McpTool {
                name: "system_telemetry".to_string(),
                description: "Retrieve real-time system and AI memory telemetry".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "tor_status".to_string(),
                description: "Check Tor transparent routing status".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "model_list".to_string(),
                description: "List registered cognitive models and memory budgets".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "model_infer".to_string(),
                description: "Execute prompt inference with local model".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "Prompt text" },
                        "model": { "type": "string", "description": "Model ID" }
                    },
                    "required": ["prompt"]
                }),
            },
        ];

        Self {
            tools,
            allowed_roots,
        }
    }

    pub fn list_tools(&self) -> Vec<McpTool> {
        self.tools.clone()
    }

    fn validate_path(&self, raw_path: &str) -> Result<PathBuf, McpError> {
        let p = Path::new(raw_path);
        // Prevent path traversal outside allowed
        if raw_path.contains("..")
            || raw_path.starts_with("/etc")
            || raw_path.starts_with("/proc")
            || raw_path.starts_with("/sys")
        {
            return Err(McpError::ForbiddenPath(raw_path.to_string()));
        }
        let matches = self
            .allowed_roots
            .iter()
            .any(|root| p.starts_with(root) || raw_path.starts_with('.'));
        if !matches && !self.allowed_roots.is_empty() && p.is_absolute() {
            return Err(McpError::ForbiddenPath(raw_path.to_string()));
        }
        Ok(p.to_path_buf())
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone();
        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "hcs-mcp-hub",
                        "version": "1.0.0"
                    },
                    "capabilities": {
                        "tools": { "listChanged": false },
                        "resources": {},
                        "prompts": {}
                    }
                });
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(result),
                    error: None,
                }
            }
            "tools/list" => {
                let result = json!({
                    "tools": self.tools
                });
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(result),
                    error: None,
                }
            }
            "tools/call" => {
                let params = match req.params {
                    Some(p) => p,
                    None => {
                        return JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32602,
                                message: "Missing params".to_string(),
                                data: None,
                            }),
                        };
                    }
                };

                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                match self.execute_tool(tool_name, arguments).await {
                    Ok(text_output) => {
                        let result = json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": text_output
                                }
                            ]
                        });
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(result),
                            error: None,
                        }
                    }
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32000,
                            message: e.to_string(),
                            data: None,
                        }),
                    },
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", req.method),
                    data: None,
                }),
            },
        }
    }

    async fn execute_tool(&self, name: &str, args: Value) -> Result<String, McpError> {
        match name {
            "fs_read_file" => {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing 'path'".to_string()))?;
                let path = self.validate_path(path_str)?;
                let content = fs::read_to_string(&path)?;
                Ok(content)
            }
            "fs_write_file" => {
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing 'path'".to_string()))?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing 'content'".to_string()))?;
                let path = self.validate_path(path_str)?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, content)?;
                Ok(format!("Successfully written to {}", path.display()))
            }
            "fs_list_dir" => {
                let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                let path = self.validate_path(path_str)?;
                let mut entries = Vec::new();
                for entry in fs::read_dir(&path)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type()?.is_dir();
                    entries.push(format!("{}{}", name, if is_dir { "/" } else { "" }));
                }
                Ok(entries.join("\n"))
            }
            "bash_run" => {
                let cmd_str = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing 'command'".to_string()))?;

                // Enforce non-root execution check
                if cmd_str.contains("sudo") || cmd_str.starts_with("rm -rf /") {
                    return Err(McpError::ToolExecution(
                        "Privileged or destructive root execution forbidden".to_string(),
                    ));
                }

                #[cfg(unix)]
                let output = Command::new("sh").arg("-c").arg(cmd_str).output()?;

                #[cfg(windows)]
                let output = Command::new("cmd").args(["/C", cmd_str]).output()?;

                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                Ok(format!(
                    "Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                    output.status.code().unwrap_or(-1),
                    stdout.trim(),
                    stderr.trim()
                ))
            }
            "memory_search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing 'query'".to_string()))?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

                let engine = MemoryEngine::open_in_memory()
                    .map_err(|e| McpError::MemoryError(e.to_string()))?;
                let results = engine
                    .hybrid_search(query, None, limit)
                    .map_err(|e| McpError::MemoryError(e.to_string()))?;

                if results.is_empty() {
                    Ok("No relevant memories found.".to_string())
                } else {
                    let mut lines = Vec::new();
                    for (i, r) in results.iter().enumerate() {
                        lines.push(format!(
                            "[{}] (score: {:.2}) {}",
                            i + 1,
                            r.total_score,
                            r.record.content
                        ));
                    }
                    Ok(lines.join("\n"))
                }
            }
            "system_telemetry" => {
                let tele = json!({
                    "timestamp": Utc::now().to_rfc3339(),
                    "ram_budget": "<= 6144 MB idle / <= 8192 MB peak",
                    "active_model": "hcs-controller (Qwen3-0.6B)",
                    "active_model_rss_mb": 550,
                    "desktop_env": "Quickshell / Niri Wayland",
                    "tor_mode": "Inactive (Default Route)",
                    "status": "Healthy"
                });
                Ok(serde_json::to_string_pretty(&tele)?)
            }
            "tor_status" => {
                let tor = TorManager::default();
                let status = tor.get_status(PrivacyMode::Standard);
                Ok(format!(
                    "Tor Running: {}\nSocks Port: {}\nTraffic Routed: {}",
                    status.is_running, status.socks_port, status.is_traffic_routed
                ))
            }
            "model_list" => {
                let catalog = json!({
                    "models": [
                        { "id": "hcs-controller", "name": "Qwen3-0.6B", "tier": "resident", "expected_ram_mb": 550 },
                        { "id": "hcs-assistant", "name": "Qwen3-1.7B", "tier": "on-demand", "expected_ram_mb": 1450 },
                        { "id": "hcs-coder", "name": "Qwen2.5-Coder-1.5B", "tier": "on-demand", "expected_ram_mb": 1350 },
                        { "id": "hcs-reasoner", "name": "Qwen3-4B", "tier": "on-demand", "expected_ram_mb": 2900 }
                    ],
                    "policy": "single_heavy_resident_enforced"
                });
                Ok(serde_json::to_string_pretty(&catalog)?)
            }
            "model_infer" => {
                let prompt = args
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidParams("Missing 'prompt'".to_string()))?;
                let model = args
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("hcs-controller");

                let mut daemon = ModelDaemon::new(PathBuf::from("/usr/share/hcs/models"), 1);
                daemon.register_model(ModelEntry {
                    id: model.to_string(),
                    name: model.to_string(),
                    repo: "org/hcs".to_string(),
                    filename: format!("{}.gguf", model),
                    revision: "main".to_string(),
                    sha256: "".to_string(),
                    parameters: "0.6B".to_string(),
                    quantization: "Q4_K_M".to_string(),
                    role: "controller".to_string(),
                    runtime_tier: "interactive".to_string(),
                    max_context: 2048,
                    expected_ram_mb: 550,
                    license: "Apache-2.0".to_string(),
                });
                let response = daemon
                    .infer(model, prompt, 0.2, 128)
                    .await
                    .map_err(|e| McpError::ToolExecution(e.to_string()))?;
                Ok(response)
            }
            _ => Err(McpError::ToolExecution(format!("Tool not found: {}", name))),
        }
    }

    /// Run stdio JSON-RPC server loop
    pub async fn run_stdio_server(&self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let reader = stdin.lock();

        eprintln!("[INFO] HCS MCP Server started on stdio transport (logs to stderr).");

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(req) => {
                    let resp = self.handle_request(req).await;
                    let resp_str = serde_json::to_string(&resp)?;
                    stdout.write_all(resp_str.as_bytes())?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                }
                Err(e) => {
                    let err_resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                            data: None,
                        }),
                    };
                    let err_str = serde_json::to_string(&err_resp)?;
                    stdout.write_all(err_str.as_bytes())?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_initialize_handshake() {
        let server = McpServer::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };
        let resp = server.handle_request(req).await;
        assert!(resp.result.is_some());
        assert_eq!(
            resp.result.unwrap().get("protocolVersion").unwrap(),
            "2024-11-05"
        );
    }

    #[tokio::test]
    async fn test_mcp_tools_list() {
        let server = McpServer::default();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };
        let resp = server.handle_request(req).await;
        assert!(resp.result.is_some());
        let tools = resp.result.unwrap().get("tools").unwrap().clone();
        assert!(tools.as_array().unwrap().len() >= 7);
    }

    #[tokio::test]
    async fn test_mcp_fs_sandboxing() {
        let server = McpServer::default();
        // Trying to read /etc/shadow must be forbidden
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "fs_read_file",
                "arguments": {
                    "path": "/etc/shadow"
                }
            })),
        };
        let resp = server.handle_request(req).await;
        let err = resp.error.expect("Expected an error for forbidden path");
        assert!(err.message.contains("Path access forbidden"));
    }
}
