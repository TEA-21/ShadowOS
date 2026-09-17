use serde_json::{json, Value};
use shadow_cli::config::ProjectConfig;
use shadow_cli::runner::{AgentRunner, SandboxContext};
use shadow_core::Result;
use shadow_cow::PatchGenerator;
use shadow_vmm::{
    ChromiumSandbox, ChromiumSandboxConfig, CdpSession, SwarmOrchestrator, VirtualDisplayConfig,
    VirtualDisplayServer,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::protocol::*;

pub struct McpHandler {
    pub workspace_root: PathBuf,
    pub sandbox_context: Arc<Mutex<SandboxContext>>,
    pub cdp_session: Arc<CdpSession>,
    pub virtual_display: Arc<Mutex<VirtualDisplayServer>>,
    pub chromium_sandbox: Arc<Mutex<ChromiumSandbox>>,
    pub swarm_orchestrator: Arc<Mutex<SwarmOrchestrator>>,
}

impl McpHandler {
    pub fn new(workspace_root: PathBuf) -> Result<Self> {
        let context = AgentRunner::initialize_sandbox(&workspace_root, None)?;

        // Initialize virtual X11 display server (Xvfb) on DISPLAY=:99 with in-memory tmpfs backing
        let mut display_server = VirtualDisplayServer::new(VirtualDisplayConfig::default());
        let _ = display_server.start();

        // Launch sandboxed headless Chromium instance with security flags and software WebGL
        let mut chromium = ChromiumSandbox::new(ChromiumSandboxConfig::default());
        let _ = chromium.start();

        // Initialize raw CDP direct session over AF_VSOCK bridge
        let cdp = CdpSession::new(2, 9222);

        // Initialize Swarm Orchestrator for concurrent worker MicroVMs
        let swarm = SwarmOrchestrator::new(workspace_root.clone());

        Ok(Self {
            workspace_root,
            sandbox_context: Arc::new(Mutex::new(context)),
            cdp_session: Arc::new(cdp),
            virtual_display: Arc::new(Mutex::new(display_server)),
            chromium_sandbox: Arc::new(Mutex::new(chromium)),
            swarm_orchestrator: Arc::new(Mutex::new(swarm)),
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
                    "instructions": "ShadowOS hardware-isolated microVM sandbox with 4-layer OverlayFS CoW protection, sub-100ms instant state rollback, Xvfb virtual display (DISPLAY=:99), raw CDP browser driving, and parallel worker swarming."
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
                    "browser_navigate" => self.call_browser_navigate(&arguments),
                    "browser_click" => self.call_browser_click(&arguments),
                    "browser_type" => self.call_browser_type(&arguments),
                    "capture_screenshot" => self.call_capture_screenshot(&arguments),
                    "swarm_spawn_worker" => self.call_swarm_spawn_worker(&arguments),
                    "swarm_dispatch_task" => self.call_swarm_dispatch_task(&arguments),
                    "swarm_collect_results" => self.call_swarm_collect_results(&arguments),
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
            ToolDefinition {
                name: "browser_navigate".to_string(),
                description: "Navigates the sandboxed headless Chromium browser (running on isolated Xvfb DISPLAY=:99) to a URL, returning page load state and title.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Target URL to navigate to"
                        },
                        "timeout_ms": {
                            "type": "integer",
                            "description": "Optional navigation timeout in milliseconds",
                            "default": 5000
                        }
                    },
                    "required": ["url"]
                }),
            },
            ToolDefinition {
                name: "browser_click".to_string(),
                description: "Simulates mouse click at coordinates (x, y) or on a CSS selector via raw Chrome DevTools Protocol (CDP).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "x": {
                            "type": "number",
                            "description": "X coordinate in viewport (0-1920)"
                        },
                        "y": {
                            "type": "number",
                            "description": "Y coordinate in viewport (0-1080)"
                        },
                        "selector": {
                            "type": "string",
                            "description": "Optional CSS selector to target"
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "browser_type".to_string(),
                description: "Simulates keyboard input into the focused DOM element or selector via raw Chrome DevTools Protocol (CDP).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text characters to type into the active element"
                        },
                        "selector": {
                            "type": "string",
                            "description": "Optional CSS selector to focus before typing"
                        }
                    },
                    "required": ["text"]
                }),
            },
            ToolDefinition {
                name: "capture_screenshot".to_string(),
                description: "Captures viewport framebuffer screenshot from the isolated in-memory Xvfb virtual display in <100ms via raw CDP.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "enum": ["png", "jpeg"],
                            "description": "Image format (default: png)",
                            "default": "png"
                        },
                        "full_page": {
                            "type": "boolean",
                            "description": "Whether to capture beyond the current 1920x1080 viewport",
                            "default": false
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "swarm_spawn_worker".to_string(),
                description: "Spawns a new concurrent worker MicroVM in the swarm with CPU core pinning, memory quotas, and branching CoW isolation.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "worker_id": {
                            "type": "string",
                            "description": "Unique identifier for the worker (e.g. worker-lint, worker-test)"
                        },
                        "cpu_core": {
                            "type": "integer",
                            "description": "Host CPU core affinity to pin this worker to (0..N)"
                        },
                        "memory_mb": {
                            "type": "integer",
                            "description": "Memory quota in megabytes (defaults to 512, idle < 200MB)",
                            "default": 512
                        }
                    },
                    "required": ["worker_id"]
                }),
            },
            ToolDefinition {
                name: "swarm_dispatch_task".to_string(),
                description: "Dispatches a sandboxed task or command to a specific swarm worker MicroVM instance.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "worker_id": {
                            "type": "string",
                            "description": "Target worker instance identifier"
                        },
                        "command": {
                            "type": "string",
                            "description": "Command or script to execute inside the worker branch"
                        },
                        "args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Command-line arguments"
                        },
                        "auto_approve": {
                            "type": "boolean",
                            "description": "Automatically inject non-interactive approval flags",
                            "default": true
                        }
                    },
                    "required": ["worker_id", "command"]
                }),
            },
            ToolDefinition {
                name: "swarm_collect_results".to_string(),
                description: "Aggregates execution reports, branching file diffs, and memory metrics across swarm workers, verifying zero cross-worker pollution.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "worker_id": {
                            "type": "string",
                            "description": "Optional specific worker ID to filter reports"
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

    fn call_browser_navigate(&self, args: &Value) -> CallToolResult {
        let url = match args.get("url").and_then(|u| u.as_str()) {
            Some(u) => u,
            None => return CallToolResult::error("Missing required 'url' argument"),
        };

        match self.cdp_session.navigate(url) {
            Ok(nav) => {
                let payload = json!({
                    "url": nav.url,
                    "title": nav.title,
                    "status": nav.status,
                    "ready_state": nav.ready_state,
                    "latency_ms": nav.latency_ms,
                    "display": ":99",
                    "resolution": "1920x1080x24",
                    "host_screen_pollution": false
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Browser navigation failed: {}", e)),
        }
    }

    fn call_browser_click(&self, args: &Value) -> CallToolResult {
        let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(100.0);
        let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(100.0);
        let selector = args.get("selector").and_then(|s| s.as_str()).unwrap_or("");

        match self.cdp_session.click(x, y) {
            Ok(_) => {
                let payload = json!({
                    "status": "clicked",
                    "coordinates": [x, y],
                    "selector": if selector.is_empty() { Value::Null } else { json!(selector) },
                    "host_screen_pollution": false
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Browser click failed: {}", e)),
        }
    }

    fn call_browser_type(&self, args: &Value) -> CallToolResult {
        let text = match args.get("text").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => return CallToolResult::error("Missing required 'text' argument"),
        };

        let selector = args.get("selector").and_then(|s| s.as_str()).unwrap_or("");

        match self.cdp_session.type_text(text) {
            Ok(count) => {
                let payload = json!({
                    "status": "typed",
                    "characters_sent": count,
                    "selector": if selector.is_empty() { Value::Null } else { json!(selector) },
                    "host_screen_pollution": false
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Browser typing failed: {}", e)),
        }
    }

    fn call_capture_screenshot(&self, args: &Value) -> CallToolResult {
        let format = args.get("format").and_then(|f| f.as_str()).unwrap_or("png");
        let full_page = args.get("full_page").and_then(|fp| fp.as_bool()).unwrap_or(false);

        match self.cdp_session.capture_screenshot(format, full_page) {
            Ok(shot) => {
                let payload = json!({
                    "format": shot.format,
                    "width": shot.width,
                    "height": shot.height,
                    "byte_size": shot.byte_size,
                    "render_latency_ms": shot.render_latency_ms,
                    "target_met": shot.target_met,
                    "data": shot.base64_data,
                    "host_screen_pollution": false
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Capture screenshot failed: {}", e)),
        }
    }

    fn call_swarm_spawn_worker(&self, args: &Value) -> CallToolResult {
        let worker_id = match args.get("worker_id").and_then(|w| w.as_str()) {
            Some(w) => w,
            None => return CallToolResult::error("Missing required 'worker_id' argument"),
        };

        let cpu_core = args.get("cpu_core").and_then(|c| c.as_u64()).map(|c| c as usize);
        let memory_mb = args.get("memory_mb").and_then(|m| m.as_u64());

        let mut swarm = self.swarm_orchestrator.lock().unwrap();
        match swarm.spawn_worker(worker_id, cpu_core, memory_mb) {
            Ok(info) => {
                let payload = json!({
                    "worker_id": info.worker_id,
                    "pinned_cpu": info.pinned_cpu,
                    "memory_quota_mb": info.memory_quota_mb,
                    "idle_footprint_mb": info.idle_footprint_mb,
                    "spawn_latency_ms": info.spawn_latency_ms,
                    "target_met": info.idle_footprint_mb < 200.0,
                    "upper_dir": info.upper_dir,
                    "status": "spawned"
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Failed to spawn swarm worker: {}", e)),
        }
    }

    fn call_swarm_dispatch_task(&self, args: &Value) -> CallToolResult {
        let worker_id = match args.get("worker_id").and_then(|w| w.as_str()) {
            Some(w) => w,
            None => return CallToolResult::error("Missing required 'worker_id' argument"),
        };

        let command = match args.get("command").and_then(|c| c.as_str()) {
            Some(c) => c,
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

        let mut swarm = self.swarm_orchestrator.lock().unwrap();
        match swarm.dispatch_task(worker_id, command, &cmd_args) {
            Ok(res) => {
                let payload = json!({
                    "task_id": res.task_id,
                    "worker_id": res.worker_id,
                    "command": res.command,
                    "exit_code": res.exit_code,
                    "stdout": res.stdout,
                    "stderr": res.stderr,
                    "duration_ms": res.duration_ms,
                    "files_modified": res.files_modified,
                    "current_memory_mb": res.current_memory_mb
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Failed to dispatch task to worker: {}", e)),
        }
    }

    fn call_swarm_collect_results(&self, args: &Value) -> CallToolResult {
        let worker_id = args.get("worker_id").and_then(|w| w.as_str());

        let swarm = self.swarm_orchestrator.lock().unwrap();
        match swarm.collect_results(worker_id) {
            Ok(summary) => {
                let payload = json!({
                    "active_workers": summary.active_workers,
                    "total_tasks_completed": summary.total_tasks_completed,
                    "aggregate_peak_memory_mb": summary.aggregate_peak_memory_mb,
                    "within_peak_memory_limit": summary.within_peak_memory_limit,
                    "zero_cross_pollution": summary.zero_cross_pollution,
                    "worker_reports": summary.worker_reports
                });
                CallToolResult::json(&payload)
            }
            Err(e) => CallToolResult::error(format!("Failed to collect swarm results: {}", e)),
        }
    }
}
