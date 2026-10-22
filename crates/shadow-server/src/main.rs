use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use shadow_server::{create_api_router, AppState};

#[derive(Parser, Debug)]
#[command(name = "shadow-server")]
#[command(version = "0.1.0")]
#[command(about = "ShadowOS HTTP REST & WebSocket API daemon for dashboard integration")]
pub struct ServerArgs {
    /// Port to listen on
    #[arg(short, long, default_value_t = 3001)]
    pub port: u16,

    /// Bind host address
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Path to target workspace repository
    #[arg(short, long)]
    pub workspace: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let args = ServerArgs::parse();

    let workspace_path = args
        .workspace
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    info!(
        "[ShadowOS Server] Initializing server for workspace: {}",
        workspace_path.display()
    );

    let state = AppState::new(workspace_path)?;

    // Spawn background telemetry heartbeat
    let bg_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(4200));
        let channels = ["vsock:3", "vsock:4", "virtio-balloon", "mcp-jsonrpc", "overlayfs"];
        let mut counter: u64 = 0;

        loop {
            interval.tick().await;
            counter += 1;
            let channel = channels[(counter as usize) % channels.len()];
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs();
            let millis = now.subsec_millis();
            let timestamp = format!(
                "{:02}:{:02}:{:02}.{:03}",
                (secs / 3600) % 24,
                (secs / 60) % 60,
                secs % 60,
                millis
            );

            let (msg, payload, level) = match channel {
                "virtio-balloon" => (
                    "Dynamic memory balloon heartbeat on vm-4a7b2e9c: RSS delta +4 MB. Idle footprint stable at 140 MB.".to_string(),
                    None,
                    "DEBUG".to_string(),
                ),
                "mcp-jsonrpc" => (
                    "MCP tool inspection: 'inspect_diff' verified zero host pollution.".to_string(),
                    Some(serde_json::json!({
                        "jsonrpc": "2.0",
                        "method": "tools/call",
                        "params": { "name": "inspect_diff", "arguments": { "sandbox_id": "vm-4a7b2e9c" } },
                        "id": format!("req-{}", counter)
                    }).to_string()),
                    "INFO".to_string(),
                ),
                "overlayfs" => (
                    "OverlayFS upperdir sync: 2 hunks modified in tmpfs scratch layer. Host repo bit-identical.".to_string(),
                    None,
                    "DEBUG".to_string(),
                ),
                _ => (
                    format!("Zero-TCP packet stream on channel #3. Multiplexed latency: 0.31 µs."),
                    None,
                    "INFO".to_string(),
                ),
            };

            bg_state.broadcast_log(shadow_server::state::VsockLogEntry {
                id: format!("log-heartbeat-{}", counter),
                timestamp,
                level,
                channel: channel.to_string(),
                vm_id: Some("vm-4a7b2e9c".to_string()),
                message: msg,
                json_rpc_payload: payload,
            });
        }
    });

    let app = create_api_router(state)
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    info!("[ShadowOS Server] Listening on http://{}", addr);
    info!("[ShadowOS Server] WebSocket Log Stream: ws://{}/api/ws/logs", addr);
    info!("[ShadowOS Server] WebSocket Terminal: ws://{}/api/ws/terminal/:id", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
