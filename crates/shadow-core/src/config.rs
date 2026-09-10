use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HypervisorType {
    Firecracker,
    Libkrun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CachePolicy {
    Always,
    Auto,
    None,
    AlwaysDax,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxMode {
    Chroot,
    Namespace,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtiofsMountConfig {
    pub tag: String,
    pub socket_path: PathBuf,
    pub shared_dir: PathBuf,
    pub cache_policy: CachePolicy,
    pub sandbox_mode: SandboxMode,
    pub thread_pool_size: usize,
    pub read_only: bool,
    pub dax_window_size_mib: u64,
}

impl Default for VirtiofsMountConfig {
    fn default() -> Self {
        Self {
            tag: "shadow-workspace".to_string(),
            socket_path: PathBuf::from("/run/shadow_virtiofs.sock"),
            shared_dir: PathBuf::from("/tmp/shadow_workspace"),
            cache_policy: CachePolicy::AlwaysDax,
            sandbox_mode: SandboxMode::Chroot,
            thread_pool_size: 4,
            read_only: true,
            dax_window_size_mib: 1024, // 1GB DAX mapping window
        }
    }
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
    pub virtiofs: VirtiofsMountConfig,
}
