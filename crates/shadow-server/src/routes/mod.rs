pub mod mcp_swarm;
pub mod snapshots;
pub mod vms;
pub mod ws;

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;

use crate::state::AppState;

pub fn create_api_router(state: AppState) -> Router {
    Router::new()
        // System health and info
        .route("/api/health", get(health_check))
        .route("/api/system/info", get(system_info))
        // MicroVM management
        .route("/api/vms", get(vms::list_vms).post(vms::spawn_vm))
        .route("/api/vms/:id/pause", post(vms::pause_vm))
        .route("/api/vms/:id/resume", post(vms::resume_vm))
        .route("/api/vms/:id", delete(vms::kill_vm))
        .route("/api/vms/:id/diff", get(snapshots::get_vm_diff))
        // Snapshots & Instant Rollback
        .route(
            "/api/snapshots",
            get(snapshots::list_snapshots).post(snapshots::create_snapshot),
        )
        .route("/api/snapshots/:id/rollback", post(snapshots::rollback_snapshot))
        // Swarm & MCP Primitives
        .route("/api/swarm/summary", get(mcp_swarm::get_swarm_summary))
        .route("/api/mcp/promote", post(mcp_swarm::promote_host_tree))
        .route("/api/mcp/call", post(mcp_swarm::call_mcp_tool))
        // WebSockets
        .route("/api/ws/logs", get(ws::ws_logs_handler))
        .route("/api/ws/terminal/:id", get(ws::ws_terminal_handler))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "shadow-server",
        "version": "0.1.0",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }))
}

async fn system_info(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed().as_secs();
    Json(crate::state::SystemInfo {
        host_os: std::env::consts::OS.to_string(),
        hypervisor_driver: state.hypervisor_type.clone(),
        kvm_available: state.kvm_available,
        workspace_root: state.workspace_root.display().to_string(),
        uptime_seconds: uptime,
        aggregate_ram_limit_mb: 2500,
        dax_enabled: true,
        protocol_version: "2024-11-05".to_string(),
    })
}
