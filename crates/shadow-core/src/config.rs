use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HypervisorType {
    Firecracker,
    Libkrun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub vcpu_count: u8,
    pub memory_size_mib: u32,
    pub peak_memory_max_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            vcpu_count: 2,
            memory_size_mib: 128,
            peak_memory_max_bytes: 2_684_354_560, // 2.5 GB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub read_only_host_mount: bool,
    pub strip_ambient_credentials: bool,
    pub isolated_network_namespace: bool,
    pub enable_seccomp: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            read_only_host_mount: true,
            strip_ambient_credentials: true,
            isolated_network_namespace: true,
            enable_seccomp: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub vm_id: String,
    pub hypervisor: HypervisorType,
    pub kernel_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub host_workspace_path: PathBuf,
    pub guest_cid: u32,
    pub vsock_port: u32,
    pub resources: ResourceLimits,
    pub security: SecurityPolicy,
}
