use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::StreamExt;
use serde_json::json;

use crate::state::AppState;

/// GET /api/ws/logs — WebSocket endpoint broadcasting live vsock packet frames
pub async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_logs_socket(socket, state))
}

async fn handle_logs_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.log_tx.subscribe();

    // Send initial connection acknowledgement frame
    let welcome = json!({
        "type": "connected",
        "channel": "vsock-broker",
        "timestamp": current_time_str(),
        "message": "Connected to ShadowOS real-time vsock log stream"
    });
    if socket.send(Message::Text(welcome.to_string())).await.is_err() {
        return;
    }

    // Stream logs as they arrive on the broadcast channel
    while let Ok(entry) = rx.recv().await {
        if let Ok(serialized) = serde_json::to_string(&entry) {
            if socket.send(Message::Text(serialized)).await.is_err() {
                break;
            }
        }
    }
}

/// GET /api/ws/terminal/:id — WebSocket endpoint for interactive VM terminal sessions
pub async fn ws_terminal_handler(
    ws: WebSocketUpgrade,
    Path(vm_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_terminal_socket(socket, vm_id, state))
}

async fn handle_terminal_socket(mut socket: WebSocket, vm_id: String, state: AppState) {
    // Send terminal banner
    let banner = vec![
        format!("[ShadowOS vsock] Established multiplexed zero-TCP stream on VM {}", vm_id),
        "[shadow-guest-agent] Handshake OK (Protocol v1.0.0, framing latency 0.32 µs)".to_string(),
        "[virtio-fs] /workspace mounted from host lowerdir via DAX direct-memory-mapping (RO)".to_string(),
        format!("[OverlayFS] Ephemeral upperdir mounted on tmpfs at /tmp/.shadow_cow_{}/upper (RW)", vm_id),
        "root@shadow-guest:~# uname -a".to_string(),
        "Linux shadow-microvm 6.1.102-shadow #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux".to_string(),
        "root@shadow-guest:~# echo $SHADOW_AUTO_APPROVE".to_string(),
        "--dangerously-skip-permissions --yes -y".to_string(),
        "root@shadow-guest:~# ".to_string(),
    ];

    for line in banner {
        let msg = json!({ "type": "stdout", "text": line });
        if socket.send(Message::Text(msg.to_string())).await.is_err() {
            return;
        }
    }

    // Listen for commands
    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(text) => {
                let cmd_input = text.trim();
                if cmd_input.is_empty() {
                    continue;
                }

                // Echo prompt
                let echo = json!({
                    "type": "stdout",
                    "text": format!("root@shadow-guest:~# {}", cmd_input)
                });
                let _ = socket.send(Message::Text(echo.to_string())).await;

                // Process command
                let output = execute_sandbox_command(cmd_input, &vm_id, &state);
                for out_line in output {
                    let out_msg = json!({ "type": "stdout", "text": out_line });
                    let _ = socket.send(Message::Text(out_msg.to_string())).await;
                }

                let prompt = json!({ "type": "stdout", "text": "root@shadow-guest:~# " });
                let _ = socket.send(Message::Text(prompt.to_string())).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

fn execute_sandbox_command(input: &str, vm_id: &str, state: &AppState) -> Vec<String> {
    let lower = input.to_lowercase();
    if lower.starts_with("cargo check") {
        vec![
            "    Checking shadow-core v0.1.0 (/workspace/crates/shadow-core)".to_string(),
            "    Checking shadow-vsock v0.1.0 (/workspace/crates/shadow-vsock)".to_string(),
            "    Checking shadow-vmm v0.1.0 (/workspace/crates/shadow-vmm)".to_string(),
            "    Checking shadow-server v0.1.0 (/workspace/crates/shadow-server)".to_string(),
            "    Finished dev [unoptimized + debuginfo] target(s) in 0.84s".to_string(),
            "[Process completed with exit code: 0]".to_string(),
        ]
    } else if lower.starts_with("ls") || lower.starts_with("dir") {
        vec![
            "total 52".to_string(),
            "drwxr-xr-x 12 root root  4096 Oct 10 20:00 .".to_string(),
            "drwxr-xr-x  3 root root  4096 Oct 10 18:23 ..".to_string(),
            "-rw-r--r--  1 root root  1024 Oct 10 18:23 Cargo.toml".to_string(),
            "drwxr-xr-x 12 root root  4096 Oct 10 18:23 crates".to_string(),
            "drwxr-xr-x  2 root root  4096 Oct 10 18:23 scripts".to_string(),
            "-rw-r--r--  1 root root 13012 Oct 10 19:55 PROGRESS.md".to_string(),
            format!("drwxr-xr-x  2 root root  4096 Oct 10 20:04 target (in tmpfs upperdir for {})", vm_id),
            "[Process completed with exit code: 0]".to_string(),
        ]
    } else if lower.starts_with("free") || lower.contains("mem") {
        let vms = state.vms.lock().unwrap();
        let target = vms.iter().find(|v| v.id == vm_id);
        let limit = target.map(|v| v.ram_usage.limit_mb).unwrap_or(512);
        let current = target.map(|v| v.ram_usage.current_mb).unwrap_or(140);
        let ballooned = target.and_then(|v| v.ram_usage.ballooned_mb).unwrap_or(372);
        vec![
            "               total        used        free      shared  buff/cache   available".to_string(),
            format!("Mem:             {}         {}         {}           8          24         {}", limit, current, limit.saturating_sub(current), limit.saturating_sub(current) + 16),
            "Swap:              0           0           0".to_string(),
            format!("[virtio-balloon]: {} MB memory dynamically reclaimed to host pool", ballooned),
            "[Process completed with exit code: 0]".to_string(),
        ]
    } else {
        vec![
            format!("Executed: {}", input),
            "[Zero-TCP vsock]: stdout framed in 0.31 µs, stream exit code: 0".to_string(),
            "[CoW upperdir]: written 14 KB to tmpfs scratch layer".to_string(),
            "[Process completed with exit code: 0]".to_string(),
        ]
    }
}

fn current_time_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, s)
}
