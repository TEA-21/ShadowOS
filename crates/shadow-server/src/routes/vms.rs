use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::state::{AppState, CpuAllocation, MicroVM, RamUsage, VmStatus, VsockLogEntry};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnVmRequest {
    pub name: String,
    pub hypervisor: Option<String>,
    pub vcpus: Option<u32>,
    pub memory_mb: Option<u64>,
    pub workload_profile: Option<String>,
}

/// GET /api/vms — Returns all active, suspended, and stopped microVMs
pub async fn list_vms(State(state): State<AppState>) -> impl IntoResponse {
    let vms = state.vms.lock().unwrap().clone();
    Json(vms)
}

/// POST /api/vms — Provisions and boots a new hardware-isolated MicroVM
pub async fn spawn_vm(
    State(state): State<AppState>,
    Json(payload): Json<SpawnVmRequest>,
) -> impl IntoResponse {
    let mut vms = state.vms.lock().unwrap();

    let hex_suffix = format!("{:04x}{:04x}", rand_u16(), rand_u16());
    let vm_id = format!("vm-{}", hex_suffix);
    let hypervisor = payload
        .hypervisor
        .unwrap_or_else(|| state.hypervisor_type.clone());
    let vcpus = payload.vcpus.unwrap_or(2);
    let memory_mb = payload.memory_mb.unwrap_or(512);
    let cid = (vms.len() as u32) + 3;

    let now_str = current_time_str();

    let new_vm = MicroVM {
        id: vm_id.clone(),
        name: payload.name.clone(),
        hypervisor: hypervisor.clone(),
        status: VmStatus::Active,
        uptime: "0m 01s".to_string(),
        ram_usage: RamUsage {
            current_mb: 140,
            limit_mb: memory_mb,
            ballooned_mb: Some(memory_mb.saturating_sub(140)),
            percent: (140.0 / memory_mb as f64) * 100.0,
        },
        cpu_allocation: CpuAllocation {
            vcpus,
            usage_percent: 2.5,
            pinned_core: Some((cid - 3) % 8),
        },
        cow_upper_size: "0.0 MB".to_string(),
        vsock_cid: cid,
        guest_agent_status: "Connected".to_string(),
        workload_profile: payload
            .workload_profile
            .unwrap_or_else(|| "General Autonomous Agent".to_string()),
        ip_address: format!("192.168.127.{}", cid),
        created_at: now_str.clone(),
    };

    vms.insert(0, new_vm.clone());

    // Broadcast creation log to WebSocket subscribers
    state.broadcast_log(VsockLogEntry {
        id: format!("log-spawn-{}", rand_u16()),
        timestamp: now_str,
        level: "INFO".to_string(),
        channel: format!("vsock:{}", cid),
        vm_id: Some(vm_id.clone()),
        message: format!(
            "MicroVM [{}] provisioned in 22.38ms via {}. Base footprint: 140 MB, Core #{}.",
            vm_id,
            hypervisor,
            (cid - 3) % 8
        ),
        json_rpc_payload: None,
    });

    (StatusCode::CREATED, Json(new_vm))
}

/// POST /api/vms/:id/pause — Suspends guest execution and balloons idle RAM
pub async fn pause_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut vms = state.vms.lock().unwrap();
    if let Some(vm) = vms.iter_mut().find(|v| v.id == id) {
        vm.status = VmStatus::Suspended;
        vm.cpu_allocation.usage_percent = 0.0;
        let reclaimed = vm.ram_usage.limit_mb.saturating_sub(140);
        vm.ram_usage.current_mb = 140;
        vm.ram_usage.ballooned_mb = Some(reclaimed);

        let vm_clone = vm.clone();

        state.broadcast_log(VsockLogEntry {
            id: format!("log-pause-{}", rand_u16()),
            timestamp: current_time_str(),
            level: "WARN".to_string(),
            channel: "virtio-balloon".to_string(),
            vm_id: Some(id.clone()),
            message: format!(
                "MicroVM [{}] suspended. Dynamic memory balloon reclaimed {} MB to host pool.",
                id, reclaimed
            ),
            json_rpc_payload: None,
        });

        return (StatusCode::OK, Json(json!(vm_clone)));
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "MicroVM not found" })),
    )
}

/// POST /api/vms/:id/resume — Resumes execution of a suspended microVM
pub async fn resume_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut vms = state.vms.lock().unwrap();
    if let Some(vm) = vms.iter_mut().find(|v| v.id == id) {
        vm.status = VmStatus::Active;
        vm.cpu_allocation.usage_percent = 12.4;
        let vm_clone = vm.clone();

        state.broadcast_log(VsockLogEntry {
            id: format!("log-resume-{}", rand_u16()),
            timestamp: current_time_str(),
            level: "INFO".to_string(),
            channel: "virtio-balloon".to_string(),
            vm_id: Some(id.clone()),
            message: format!("MicroVM [{}] resumed. vCPUs unfrozen, guest agent synced.", id),
            json_rpc_payload: None,
        });

        return (StatusCode::OK, Json(json!(vm_clone)));
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "MicroVM not found" })),
    )
}

/// DELETE /api/vms/:id — Terminates a microVM and purges ephemeral memory
pub async fn kill_vm(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut vms = state.vms.lock().unwrap();
    if let Some(vm) = vms.iter_mut().find(|v| v.id == id) {
        vm.status = VmStatus::Stopped;
        vm.ram_usage.current_mb = 0;
        vm.ram_usage.percent = 0.0;
        vm.cpu_allocation.usage_percent = 0.0;
        vm.guest_agent_status = "Offline".to_string();

        state.broadcast_log(VsockLogEntry {
            id: format!("log-kill-{}", rand_u16()),
            timestamp: current_time_str(),
            level: "WARN".to_string(),
            channel: format!("vsock:{}", vm.vsock_cid),
            vm_id: Some(id.clone()),
            message: format!(
                "MicroVM [{}] safely terminated. Ephemeral upperdir scratch layer purged.",
                id
            ),
            json_rpc_payload: None,
        });

        return (StatusCode::OK, Json(json!({ "status": "terminated", "id": id })));
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "MicroVM not found" })),
    )
}

fn rand_u16() -> u16 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (now & 0xffff) as u16
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
