use async_trait::async_trait;
use shadow_core::{Result, VmConfig};

#[async_trait]
pub trait VMMDriver: Send + Sync {
    /// Initialize the hypervisor environment, validate kernel/rootfs, and configure devices
    async fn init(&mut self, config: &VmConfig) -> Result<()>;

    /// Boot the MicroVM instance (< 150ms cold boot target)
    async fn start(&mut self) -> Result<()>;

    /// Pause the MicroVM VCPUs for checkpointing or idle preservation
    async fn pause(&mut self) -> Result<()>;

    /// Resume execution of paused MicroVM VCPUs
    async fn resume(&mut self) -> Result<()>;

    /// Stop and terminate the MicroVM, releasing hypervisor and cgroup resources
    async fn stop(&mut self) -> Result<()>;

    /// Query the health status of the running MicroVM
    async fn is_alive(&self) -> Result<bool>;

    /// Create a memory and device state snapshot to the specified path
    async fn snapshot(&self, mem_file_path: &std::path::Path, state_file_path: &std::path::Path) -> Result<()>;

    /// Restore a memory and device state snapshot from the specified path
    async fn restore(&self, mem_file_path: &std::path::Path, state_file_path: &std::path::Path) -> Result<()>;
}
