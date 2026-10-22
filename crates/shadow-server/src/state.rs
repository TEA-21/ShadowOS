use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::broadcast;

use shadow_mcp::McpHandler;
use shadow_snapshot::CheckpointOrchestrator;
use shadow_vmm::SwarmOrchestrator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VmStatus {
    Active,
    Suspended,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RamUsage {
    pub current_mb: u64,
    pub limit_mb: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ballooned_mb: Option<u64>,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuAllocation {
    pub vcpus: u32,
    pub usage_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_core: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicroVM {
    pub id: string_or_id::Id,
    pub name: String,
    pub hypervisor: String, // "libkrun" | "firecracker"
    pub status: VmStatus,
    pub uptime: String,
    pub ram_usage: RamUsage,
    pub cpu_allocation: CpuAllocation,
    pub cow_upper_size: String,
    pub vsock_cid: u32,
    pub guest_agent_status: String, // "Connected" | "Syncing" | "Offline"
    pub workload_profile: String,
    pub ip_address: String,
    pub created_at: String,
}

mod string_or_id {
    pub type Id = String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    #[serde(rename = "type")]
    pub line_type: String, // "add" | "delete" | "context" | "hunk"
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_line_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_line_number: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_words: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffItem {
    pub path: String,
    pub status: String, // "added" | "modified" | "deleted"
    pub additions: usize,
    pub deletions: usize,
    pub layer: String,
    pub diff_lines: Vec<DiffLine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_explanation: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotStats {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub total_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub vm_id: String,
    pub name: String,
    pub timestamp: String,
    pub date: String,
    pub description: String,
    pub trigger: String, // "manual" | "pre-command" | "swarm-checkpoint" | "milestone"
    pub ram_footprint_mb: u64,
    pub cow_upper_size: String,
    pub rollback_latency_ms: f64,
    pub stats: SnapshotStats,
    pub files: Vec<FileDiffItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall_ai_summary: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VsockLogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String, // "INFO" | "TRACE" | "DEBUG" | "WARN" | "ERROR"
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_id: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_rpc_payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNoteMetrics {
    pub test_passed: usize,
    pub test_total: usize,
    pub compilation_time_ms: u64,
    pub uncommitted_diff_bytes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NfAudit {
    pub id: String,
    pub label: String,
    pub target: String,
    pub measured: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNote {
    pub id: String,
    pub agent_id: String,
    pub title: String,
    pub target_task: String,
    pub timestamp: String,
    pub status: String, // "In Progress" | "Completed" | "Review Required"
    pub metrics: AgentNoteMetrics,
    pub coordinator: String,
    pub hypervisor_driver: String,
    pub filesystem_status: String,
    pub sandboxing_policy: String,
    pub nfr_audits: Vec<NfAudit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub host_os: String,
    pub hypervisor_driver: String,
    pub kvm_available: bool,
    pub workspace_root: String,
    pub uptime_seconds: u64,
    pub aggregate_ram_limit_mb: u64,
    pub dax_enabled: bool,
    pub protocol_version: String,
}

#[derive(Clone)]
pub struct AppState {
    pub workspace_root: PathBuf,
    pub start_time: Instant,
    pub hypervisor_type: String,
    pub kvm_available: bool,
    pub vms: Arc<Mutex<Vec<MicroVM>>>,
    pub snapshots: Arc<Mutex<Vec<Snapshot>>>,
    pub agent_notes: Arc<Mutex<Vec<AgentNote>>>,
    pub log_tx: broadcast::Sender<VsockLogEntry>,
    pub mcp_handler: Arc<McpHandler>,
    pub checkpoint_orchestrator: Arc<Mutex<CheckpointOrchestrator>>,
    pub swarm_orchestrator: Arc<Mutex<SwarmOrchestrator>>,
}

impl AppState {
    pub fn new(workspace_root: PathBuf) -> anyhow::Result<Self> {
        let (log_tx, _) = broadcast::channel(500);

        // Detect KVM / hypervisor availability
        let kvm_available = cfg!(target_os = "linux") && std::path::Path::new("/dev/kvm").exists();
        let hypervisor_type = if kvm_available {
            "libkrun".to_string()
        } else {
            "mock".to_string()
        };

        // Initialize McpHandler
        let mcp_handler = Arc::new(McpHandler::new(workspace_root.clone())?);

        // Initialize CheckpointOrchestrator
        let shm_dir = workspace_root.join(".shadow").join("shm_checkpoints");
        let _ = std::fs::create_dir_all(&shm_dir);
        let checkpoint_orchestrator = Arc::new(Mutex::new(CheckpointOrchestrator::new(shm_dir)));

        // Initialize SwarmOrchestrator
        let swarm_orchestrator = Arc::new(Mutex::new(SwarmOrchestrator::new(workspace_root.clone())));

        // Initialize default MicroVMs
        let initial_vms = vec![
            MicroVM {
                id: "vm-4a7b2e9c".to_string(),
                name: "agent-workspace-primary".to_string(),
                hypervisor: "libkrun".to_string(),
                status: VmStatus::Active,
                uptime: "1h 42m 18s".to_string(),
                ram_usage: RamUsage {
                    current_mb: 140,
                    limit_mb: 512,
                    ballooned_mb: Some(372),
                    percent: 27.3,
                },
                cpu_allocation: CpuAllocation {
                    vcpus: 2,
                    usage_percent: 14.2,
                    pinned_core: Some(0),
                },
                cow_upper_size: "18.4 MB".to_string(),
                vsock_cid: 3,
                guest_agent_status: "Connected".to_string(),
                workload_profile: "Refactoring / Code Review".to_string(),
                ip_address: "192.168.127.2".to_string(),
                created_at: "2026-10-10 14:22:00".to_string(),
            },
            MicroVM {
                id: "vm-9f3d1c8a".to_string(),
                name: "agent-eval-worker-01".to_string(),
                hypervisor: "firecracker".to_string(),
                status: VmStatus::Active,
                uptime: "48m 10s".to_string(),
                ram_usage: RamUsage {
                    current_mb: 280,
                    limit_mb: 1024,
                    ballooned_mb: Some(744),
                    percent: 27.3,
                },
                cpu_allocation: CpuAllocation {
                    vcpus: 4,
                    usage_percent: 68.5,
                    pinned_core: Some(1),
                },
                cow_upper_size: "42.1 MB".to_string(),
                vsock_cid: 4,
                guest_agent_status: "Connected".to_string(),
                workload_profile: "Test Suite Execution (pytest)".to_string(),
                ip_address: "192.168.127.3".to_string(),
                created_at: "2026-10-10 15:16:08".to_string(),
            },
            MicroVM {
                id: "vm-6c2e8b0f".to_string(),
                name: "agent-linter-ephemeral".to_string(),
                hypervisor: "libkrun".to_string(),
                status: VmStatus::Suspended,
                uptime: "12m 44s".to_string(),
                ram_usage: RamUsage {
                    current_mb: 140,
                    limit_mb: 256,
                    ballooned_mb: Some(116),
                    percent: 54.6,
                },
                cpu_allocation: CpuAllocation {
                    vcpus: 1,
                    usage_percent: 0.0,
                    pinned_core: Some(2),
                },
                cow_upper_size: "2.3 MB".to_string(),
                vsock_cid: 5,
                guest_agent_status: "Syncing".to_string(),
                workload_profile: "Clippy / Static Analysis".to_string(),
                ip_address: "192.168.127.4".to_string(),
                created_at: "2026-10-10 15:52:00".to_string(),
            },
            MicroVM {
                id: "vm-9e32b4a1".to_string(),
                name: "agent-headless-browser".to_string(),
                hypervisor: "libkrun".to_string(),
                status: VmStatus::Active,
                uptime: "28m 15s".to_string(),
                ram_usage: RamUsage {
                    current_mb: 220,
                    limit_mb: 1024,
                    ballooned_mb: Some(804),
                    percent: 21.5,
                },
                cpu_allocation: CpuAllocation {
                    vcpus: 2,
                    usage_percent: 24.8,
                    pinned_core: Some(2),
                },
                cow_upper_size: "14.2 MB".to_string(),
                vsock_cid: 6,
                guest_agent_status: "Connected".to_string(),
                workload_profile: "Xvfb (DISPLAY=:99) + Raw CDP Bridge".to_string(),
                ip_address: "192.168.127.5".to_string(),
                created_at: "2026-10-10 16:10:00".to_string(),
            },
            MicroVM {
                id: "vm-1c70e884".to_string(),
                name: "agent-swarm-coordinator".to_string(),
                hypervisor: "firecracker".to_string(),
                status: VmStatus::Active,
                uptime: "54m 30s".to_string(),
                ram_usage: RamUsage {
                    current_mb: 140,
                    limit_mb: 512,
                    ballooned_mb: Some(372),
                    percent: 27.3,
                },
                cpu_allocation: CpuAllocation {
                    vcpus: 1,
                    usage_percent: 5.4,
                    pinned_core: Some(3),
                },
                cow_upper_size: "0.0 MB".to_string(),
                vsock_cid: 7,
                guest_agent_status: "Connected".to_string(),
                workload_profile: "Swarm Worker Dispatch & JSON-RPC Loop".to_string(),
                ip_address: "192.168.127.6".to_string(),
                created_at: "2026-10-10 16:30:00".to_string(),
            },
        ];

        // Initialize baseline snapshots
        let initial_snapshots = vec![
            Snapshot {
                id: "snap-001-baseline".to_string(),
                vm_id: "vm-4a7b2e9c".to_string(),
                name: "Pre-Refactor Checkpoint".to_string(),
                timestamp: "14:30:12".to_string(),
                date: "2026-10-10".to_string(),
                description: "Baseline memory snapshot before Claude began refactoring virtio-fs DAX handler.".to_string(),
                trigger: "pre-command".to_string(),
                ram_footprint_mb: 140,
                cow_upper_size: "0.0 MB".to_string(),
                rollback_latency_ms: 22.38,
                stats: SnapshotStats {
                    added: 0,
                    modified: 0,
                    deleted: 0,
                    total_files: 0,
                },
                files: vec![],
                overall_ai_summary: Some(vec![
                    "Clean baseline captured before agent execution.".to_string(),
                    "Host filesystem verified bit-identical via SHA256 checksum.".to_string(),
                ]),
            },
            Snapshot {
                id: "snap-002-test-failure".to_string(),
                vm_id: "vm-4a7b2e9c".to_string(),
                name: "Broken Test State (Rollback Ready)".to_string(),
                timestamp: "15:05:40".to_string(),
                date: "2026-10-10".to_string(),
                description: "Automatic snapshot taken after test suite returned exit code 1.".to_string(),
                trigger: "swarm-checkpoint".to_string(),
                ram_footprint_mb: 285,
                cow_upper_size: "18.4 MB".to_string(),
                rollback_latency_ms: 31.05,
                stats: SnapshotStats {
                    added: 1,
                    modified: 3,
                    deleted: 1,
                    total_files: 5,
                },
                files: vec![
                    FileDiffItem {
                        path: "crates/shadow-vmm/src/virtiofs.rs".to_string(),
                        status: "modified".to_string(),
                        additions: 14,
                        deletions: 6,
                        layer: "upperdir (tmpfs RW)".to_string(),
                        diff_lines: vec![
                            DiffLine {
                                line_type: "hunk".to_string(),
                                text: "@@ -112,6 +112,14 @@ impl VirtiofsDaemon {".to_string(),
                                old_line_number: None,
                                new_line_number: None,
                                highlight_words: None,
                            },
                            DiffLine {
                                line_type: "context".to_string(),
                                text: "     pub fn configure_dax(&mut self) -> Result<()> {".to_string(),
                                old_line_number: Some(112),
                                new_line_number: Some(112),
                                highlight_words: None,
                            },
                            DiffLine {
                                line_type: "delete".to_string(),
                                text: "-        self.cache_policy = CachePolicy::Never;".to_string(),
                                old_line_number: Some(113),
                                new_line_number: None,
                                highlight_words: None,
                            },
                            DiffLine {
                                line_type: "add".to_string(),
                                text: "+        self.cache_policy = CachePolicy::Always;".to_string(),
                                old_line_number: None,
                                new_line_number: Some(113),
                                highlight_words: None,
                            },
                            DiffLine {
                                line_type: "add".to_string(),
                                text: "+        self.enable_dax_window(2048)?;".to_string(),
                                old_line_number: None,
                                new_line_number: Some(114),
                                highlight_words: None,
                            },
                        ],
                        ai_explanation: Some(vec![
                            "Switched cache policy to Always and opened a 2048 MB DAX memory window.".to_string(),
                        ]),
                    },
                ],
                overall_ai_summary: Some(vec![
                    "Agent modified cache policy in virtiofs.rs and added tests.".to_string(),
                    "Ready for instant sub-100ms rollback or promotion to host tree.".to_string(),
                ]),
            },
            Snapshot {
                id: "snap-003-cdp-session".to_string(),
                vm_id: "vm-9e32b4a1".to_string(),
                name: "Headless Browser Virtual Display Checkpoint".to_string(),
                timestamp: "16:12:05".to_string(),
                date: "2026-10-10".to_string(),
                description: "In-memory Xvfb virtual display snapshot captured before autonomous agent executed raw CDP commands.".to_string(),
                trigger: "milestone".to_string(),
                ram_footprint_mb: 220,
                cow_upper_size: "14.2 MB".to_string(),
                rollback_latency_ms: 28.40,
                stats: SnapshotStats {
                    added: 2,
                    modified: 1,
                    deleted: 0,
                    total_files: 3,
                },
                files: vec![
                    FileDiffItem {
                        path: "crates/shadow-vmm/src/cdp.rs".to_string(),
                        status: "modified".to_string(),
                        additions: 22,
                        deletions: 4,
                        layer: "upperdir (tmpfs RW)".to_string(),
                        diff_lines: vec![
                            DiffLine {
                                line_type: "hunk".to_string(),
                                text: "@@ -45,4 +45,12 @@ impl CdpBridge {".to_string(),
                                old_line_number: None,
                                new_line_number: None,
                                highlight_words: None,
                            },
                            DiffLine {
                                line_type: "context".to_string(),
                                text: "    pub async fn send_command(&mut self, method: &str) -> Result<()> {".to_string(),
                                old_line_number: Some(45),
                                new_line_number: Some(45),
                                highlight_words: None,
                            },
                            DiffLine {
                                line_type: "add".to_string(),
                                text: "+        let req = json!({\"method\": method, \"id\": self.next_id()});".to_string(),
                                old_line_number: None,
                                new_line_number: Some(46),
                                highlight_words: None,
                            },
                        ],
                        ai_explanation: Some(vec![
                            "Added raw JSON-RPC DevTools Protocol framing over AF_VSOCK port 9222.".to_string(),
                        ]),
                    },
                ],
                overall_ai_summary: Some(vec![
                    "Virtual display initialized on DISPLAY=:99 with sub-millisecond viewport capture.".to_string(),
                ]),
            },
        ];

        let initial_notes = vec![
            AgentNote {
                id: "note-01".to_string(),
                agent_id: "agent-workspace-primary".to_string(),
                title: "Refactor virtio-fs DAX Direct Memory Mapping".to_string(),
                target_task: "Implement sub-millisecond DAX memory window mapping and OverlayFS 4-layer CoW stack.".to_string(),
                timestamp: "2026-10-10 15:45:00".to_string(),
                status: "In Progress".to_string(),
                metrics: AgentNoteMetrics {
                    test_passed: 16,
                    test_total: 16,
                    compilation_time_ms: 840,
                    uncommitted_diff_bytes: "18.4 KB".to_string(),
                },
                coordinator: "ShadowOS MCP Swarm Orchestrator".to_string(),
                hypervisor_driver: "libkrun (Hardware Virtualization / KVM)".to_string(),
                filesystem_status: "OverlayFS 4-Layer CoW Active (Host lowerdir untouched)".to_string(),
                sandboxing_policy: "--dangerously-skip-permissions --yes -y (Auto-injected)".to_string(),
                nfr_audits: vec![
                    NfAudit {
                        id: "NFR-01".to_string(),
                        label: "Cold Boot Provisioning Latency".to_string(),
                        target: "< 150.0 ms".to_string(),
                        measured: "22.38 ms".to_string(),
                        passed: true,
                    },
                    NfAudit {
                        id: "NFR-02".to_string(),
                        label: "Base Idle RAM Footprint".to_string(),
                        target: "< 150.0 MB".to_string(),
                        measured: "140.0 MB".to_string(),
                        passed: true,
                    },
                    NfAudit {
                        id: "NFR-04".to_string(),
                        label: "virtio-fs Read Throughput".to_string(),
                        target: ">= 85.0% Native".to_string(),
                        measured: "130.75%".to_string(),
                        passed: true,
                    },
                    NfAudit {
                        id: "P2-02".to_string(),
                        label: "Instant Rollback Latency".to_string(),
                        target: "< 100.0 ms".to_string(),
                        measured: "31.05 ms".to_string(),
                        passed: true,
                    },
                ],
            },
            AgentNote {
                id: "note-02".to_string(),
                agent_id: "agent-swarm-coordinator".to_string(),
                title: "Swarm Worker Affinity & Dynamic Balloon Clamping".to_string(),
                target_task: "Pin concurrent workers (0..3), verify zero cross-agent pollution, enforce aggregate quota <= 2.5GB.".to_string(),
                timestamp: "2026-10-10 16:40:00".to_string(),
                status: "Completed".to_string(),
                metrics: AgentNoteMetrics {
                    test_passed: 24,
                    test_total: 24,
                    compilation_time_ms: 410,
                    uncommitted_diff_bytes: "0 KB".to_string(),
                },
                coordinator: "ShadowOS Swarm Supervisor".to_string(),
                hypervisor_driver: "Firecracker (MicroVM Isolation)".to_string(),
                filesystem_status: "Branching CoW Upperdir Verified Bit-Identical".to_string(),
                sandboxing_policy: "--dangerously-skip-permissions --yes -y".to_string(),
                nfr_audits: vec![
                    NfAudit {
                        id: "M3-13".to_string(),
                        label: "Swarm Worker Provisioning".to_string(),
                        target: "< 150.0 ms".to_string(),
                        measured: "0.89 ms avg".to_string(),
                        passed: true,
                    },
                    NfAudit {
                        id: "M3-14".to_string(),
                        label: "virtio-balloon Reclamation".to_string(),
                        target: "< 200 MB idle".to_string(),
                        measured: "140.0 MB".to_string(),
                        passed: true,
                    },
                    NfAudit {
                        id: "M3-16".to_string(),
                        label: "Aggregate Swarm RAM Quota".to_string(),
                        target: "<= 2.50 GB".to_string(),
                        measured: "1.55 GB peak".to_string(),
                        passed: true,
                    },
                ],
            },
        ];

        Ok(Self {
            workspace_root,
            start_time: Instant::now(),
            hypervisor_type,
            kvm_available,
            vms: Arc::new(Mutex::new(initial_vms)),
            snapshots: Arc::new(Mutex::new(initial_snapshots)),
            agent_notes: Arc::new(Mutex::new(initial_notes)),
            log_tx,
            mcp_handler,
            checkpoint_orchestrator,
            swarm_orchestrator,
        })
    }

    pub fn broadcast_log(&self, entry: VsockLogEntry) {
        let _ = self.log_tx.send(entry);
    }
}
