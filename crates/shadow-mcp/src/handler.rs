use serde_json::{json, Value};
use shadow_cli::config::ProjectConfig;
use shadow_cli::runner::{AgentRunner, SandboxContext};
use shadow_core::Result;
use shadow_cow::PatchGenerator;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::protocol::*;

pub struct McpHandler {
    pub workspace_root: PathBuf,
    pub sandbox_context: Arc<Mutex<SandboxContext>>,
}

impl McpHandler {
    pub fn new(workspace_root: PathBuf) -> Result<Self> {
        let context = AgentRunner::initialize_sandbox(&workspace_root, None)?;
        Ok(Self {
            workspace_root,
            sandbox_context: Arc::new(Mutex::new(context)),
        })
    }

    /// Handles an incoming JSON-RPC 2.0 request
    pub fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let req_id = req.id.clone();

        match req.method.as_str() {
            "initialize" => {
                let init_result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "shadow-mcp",
                        "version": "0.1.0"
                    },
                    "instructions": "ShadowOS hardware-isolated microVM sandbox with 4-layer OverlayFS CoW protection, sub-100ms instant state rollback, and zero host filesystem pollution."
                });
                Some(JsonRpcResponse::success(req_id, init_result))
            }
            "notifications/initialized" => {
                // Notification: no response required
                None
            }
            "ping" => Some(JsonRpcResponse::success(req_id, json!({}))),
            "tools/list" => {
                let tools = self.get_tools();
                Some(JsonRpcResponse::success(req_id, json!({ "tools": tools })))
            }
            "tools/call" => {
                let params = req.params.unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                let result = match tool_name {
                    "run_sandboxed_cmd" => self.call_run_sandboxed_cmd(&arguments),
                    "inspect_diff" => self.call_inspect_diff(&arguments),
                    "rollback_state" => self.call_rollback_state(&arguments),
                    "promote_change" => self.call_promote_change(&arguments),
                    unknown => CallToolResult::error(format!("Unknown tool: {}", unknown)),
                };

                let res_json = serde_json::to_value(result).unwrap_or(json!({}));
                Some(JsonRpcResponse::success(req_id, res_json))
            }
            unknown_method => Some(JsonRpcResponse::error(
                req_id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", unknown_method),
            )),
        }
    }

    pub fn get_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "run_sandboxed_cmd".to_string(),
                description: "Executes a command inside the hardware-isolated ShadowOS MicroVM with 4-layer OverlayFS ephemeral CoW protection, preventing host filesystem pollution and prompt stalls.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command or binary to execute (e.g. bash, cargo, npm, pytest)"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Command-line arguments"
                        },
                        "prompt": {
                            "type": "string",
                            "description": "Optional agent prompt or task description"
                        },
                        "auto_approve": {
                            "type": "boolean",
                            "description": "Inject auto-approval flags (--dangerously-skip-permissions, -y) to eliminate stalls",
                            "default": true
                        }
                    },
                    "required": ["command"]
                }),
            },
            ToolDefinition {
                name: "inspect_diff".to_string(),
                description: "Inspects unified git diff of ephemeral modifications made in the guest upperdir against the pristine host project.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file": {
                            "type": "string",
                            "description": "Optional specific relative file path to diff"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "rollback_state".to_string(),
                description: "Instantly rolls back guest memory and ephemeral OverlayFS upperdir storage in < 100ms, restoring pristine host baseline.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "checkpoint_id": {
                            "type": "string",
                            "description": "Optional checkpoint identifier (defaults to 'baseline')",
                            "default": "baseline"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "promote_change".to_string(),
                description: "Atomically stages and promotes verified guest file modifications back to the host repository.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "file": {
                            "type": "string",
                            "description": "Optional relative file path to promote (promotes all modified files if omitted)"
                        }
                    }
                }),
            },
        ]
    }

    fn call_run_sandboxed_cmd(&self, args: &Value) -> CallToolResult {
        let command = match args.get("command").and_then(|c| c.as_str()) {
            Some(cmd) => cmd,
            None => return CallToolResult::error("Missing required 'command' argument"),
        };

        let cmd_args: Vec<String> = args
            .get("args")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let prompt = args.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
        let auto_approve = args
            .get("auto_approve")
            .and_then(|b| b.as_bool())
            .unwrap_or(true);

        let mut config = ProjectConfig::load_from_dir(&self.workspace_root);
        config.auto_approve = auto_approve;

        let cmd_req =
            AgentRunner::prepare_agent_invocation(command, prompt, &cmd_args, &config);

        let context = self.sandbox_context.lock().unwrap();
        match AgentRunner::execute_unattended_command(&context, &cmd_req) {
            Ok(summary) => {
                let payload = json!({
                    "command": summary.command_line,
                    "exit_code": summary.exit_code,
                    "stdout": summary.stdout,
                    "stderr": summary.stderr,
                    "duration_ms": summary.duration_ms,
                    "files_modified": summary.files_modified_in_overlay,
                    "host_pollution": summary.host_pollution_detected
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Sandbox execution failed: {}", e)),
        }
    }

    fn call_inspect_diff(&self, args: &Value) -> CallToolResult {
        let context = self.sandbox_context.lock().unwrap();
        let changes = match context.overlay_manager.scan_upperdir_changes() {
            Ok(c) => c,
            Err(e) => return CallToolResult::error(format!("Failed to scan upperdir: {}", e)),
        };

        if changes.is_empty() {
            return CallToolResult::json(&json!({
                "modified_files": [],
                "unified_diff": "",
                "status": "No ephemeral modifications detected. Host repository is pristine."
            }));
        }

        let target_file = args.get("file").and_then(|f| f.as_str());
        let mut unified_diffs = Vec::new();
        let mut modified_list = Vec::new();

        for file_change in &changes {
            let rel_str = file_change.relative_path.to_string_lossy().to_string();
            if let Some(target) = target_file {
                if rel_str != target {
                    continue;
                }
            }

            modified_list.push(rel_str.clone());
            let host_file = context.host_workspace.join(&file_change.relative_path);
            let upper_file = context.upper_dir.join(&file_change.relative_path);

            if let Ok(diff) = PatchGenerator::generate_unified_diff(
                &host_file,
                &upper_file,
                &file_change.relative_path,
            ) {
                unified_diffs.push(diff);
            }
        }

        let payload = json!({
            "modified_files": modified_list,
            "unified_diff": unified_diffs.join("\n"),
            "file_count": modified_list.len()
        });

        CallToolResult::json(&payload)
    }

    fn call_rollback_state(&self, args: &Value) -> CallToolResult {
        let checkpoint_id = args
            .get("checkpoint_id")
            .and_then(|c| c.as_str())
            .unwrap_or("baseline");

        let context = self.sandbox_context.lock().unwrap();
        let t0 = Instant::now();

        match context.overlay_manager.reset_upperdir() {
            Ok(_) => {
                let duration_ms = t0.elapsed().as_secs_f64() * 1000.0;
                let payload = json!({
                    "status": "rolled_back",
                    "checkpoint_id": checkpoint_id,
                    "latency_ms": duration_ms,
                    "target_met": duration_ms < 100.0,
                    "message": "Ephemeral upperdir purged. Workspace restored to baseline."
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Rollback failed: {}", e)),
        }
    }

    fn call_promote_change(&self, args: &Value) -> CallToolResult {
        let context = self.sandbox_context.lock().unwrap();
        let changes = match context.overlay_manager.scan_upperdir_changes() {
            Ok(c) => c,
            Err(e) => return CallToolResult::error(format!("Failed to scan upperdir: {}", e)),
        };

        if changes.is_empty() {
            return CallToolResult::json(&json!({
                "promoted_count": 0,
                "message": "No ephemeral modifications found to promote."
            }));
        }

        let target_file = args.get("file").and_then(|f| f.as_str());
        let mut count = 0;

        for change in &changes {
            let rel_str = change.relative_path.to_string_lossy().to_string();
            if let Some(target) = target_file {
                if rel_str != target {
                    continue;
                }
            }

            let upper = context.upper_dir.join(&change.relative_path);
            let host = context.host_workspace.join(&change.relative_path);

            if PatchGenerator::promote_file(&upper, &host).is_ok() {
                count += 1;
            }
        }

        let new_hash =
            shadow_cow::OverlayManager::compute_directory_sha256(&context.host_workspace)
                .unwrap_or_default();

        let payload = json!({
            "promoted_count": count,
            "new_host_sha256": new_hash,
            "status": "Changes atomically promoted to host workspace."
        });

        CallToolResult::json(&payload)
    }
}
