use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use shadow_core::{Result, ShadowError, VmConfig};
use crate::traits::VMMDriver;

pub struct FirecrackerDriver {
    config: Option<VmConfig>,
    jailer_child: Option<Child>,
    socket_path: PathBuf,
}

impl FirecrackerDriver {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            config: None,
            jailer_child: None,
            socket_path,
        }
    }

    /// Spawns the Firecracker process wrapped in the security jailer
    async fn spawn_jailer(&mut self, config: &VmConfig) -> Result<()> {
        let jailer_bin = std::env::var("FIRECRACKER_JAILER_BIN")
            .unwrap_or_else(|_| "/usr/local/bin/jailer".to_string());
        let firecracker_bin = std::env::var("FIRECRACKER_BIN")
            .unwrap_or_else(|_| "/usr/local/bin/firecracker".to_string());

        let mut cmd = Command::new(jailer_bin);
        cmd.arg("--id").arg(&config.vm_id)
            .arg("--exec-file").arg(&firecracker_bin)
            .arg("--uid").arg("10001")
            .arg("--gid").arg("10001")
            .arg("--chroot-base-dir").arg("/srv/shadowos/jail")
            .arg("--")
            .arg("--api-sock").arg(&self.socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::info!("Launching Firecracker jailer for VM ID: {}", config.vm_id);
        let child = cmd.spawn().map_err(|e| {
            ShadowError::Vmm(format!("Failed to spawn Firecracker jailer: {}", e))
        })?;

        self.jailer_child = Some(child);
        Ok(())
    }

    /// Helper to send JSON-RPC / REST calls over Firecracker UDS
    async fn send_api_request(&self, method: &str, endpoint: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value> {
        tracing::debug!("Firecracker API {} {} - Body: {:?}", method, endpoint, body);
        // Simulation / Unix Domain Socket HTTP client bridge
        Ok(serde_json::json!({ "status": "ok" }))
    }
}

#[async_trait]
impl VMMDriver for FirecrackerDriver {
    async fn init(&mut self, config: &VmConfig) -> Result<()> {
        self.config = Some(config.clone());
        self.spawn_jailer(config).await?;

        // 1. Configure Boot Source (uncompressed vmlinux, quiet, nomodules, direct init)
        let boot_source = serde_json::json!({
            "kernel_image_path": config.kernel_path.to_string_lossy(),
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off nomodules quiet lpj=100000 init=/sbin/shadow-guest-agent"
        });
        self.send_api_request("PUT", "/boot-source", Some(boot_source)).await?;

        // 2. Configure Machine Resources
        let machine_config = serde_json::json!({
            "vcpu_count": config.resources.vcpu_count,
            "mem_size_mib": config.resources.memory_size_mib,
            "ht_enabled": false
        });
        self.send_api_request("PUT", "/machine-config", Some(machine_config)).await?;

        // 3. Configure Ephemeral Rootfs Drive
        let rootfs_drive = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": config.rootfs_path.to_string_lossy(),
            "is_root_device": true,
            "is_read_only": true
        });
        self.send_api_request("PUT", "/drives/rootfs", Some(rootfs_drive)).await?;

        // 4. Configure AF_VSOCK Device
        let vsock_dev = serde_json::json!({
            "vsock_id": "vsock0",
            "guest_cid": config.guest_cid,
            "uds_path": format!("/run/shadow_{}.vsock", config.vm_id)
        });
        self.send_api_request("PUT", "/vsock", Some(vsock_dev)).await?;

        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        let action = serde_json::json!({ "action_type": "InstanceStart" });
        self.send_api_request("PUT", "/actions", Some(action)).await?;
        tracing::info!("Firecracker MicroVM booted successfully.");
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        let state = serde_json::json!({ "state": "Paused" });
        self.send_api_request("PATCH", "/vm", Some(state)).await?;
        tracing::info!("Firecracker MicroVM paused.");
        Ok(())
    }

    async fn resume(&mut self) -> Result<()> {
        let state = serde_json::json!({ "state": "Resumed" });
        self.send_api_request("PATCH", "/vm", Some(state)).await?;
        tracing::info!("Firecracker MicroVM resumed.");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let action = serde_json::json!({ "action_type": "SendCtrlAltDel" });
        let _ = self.send_api_request("PUT", "/actions", Some(action)).await;
        if let Some(mut child) = self.jailer_child.take() {
            let _ = child.kill().await;
        }
        tracing::info!("Firecracker MicroVM stopped and cleaned up.");
        Ok(())
    }

    async fn is_alive(&self) -> Result<bool> {
        let res = self.send_api_request("GET", "/describe", None).await;
        Ok(res.is_ok())
    }

    async fn snapshot(&self, mem_file_path: &Path, state_file_path: &Path) -> Result<()> {
        self.pause().await?;
        let snap_payload = serde_json::json!({
            "snapshot_type": "Diff",
            "snapshot_path": state_file_path.to_string_lossy(),
            "mem_file_path": mem_file_path.to_string_lossy(),
        });
        self.send_api_request("PUT", "/snapshot/create", Some(snap_payload)).await?;
        self.resume().await?;
        tracing::info!("Snapshot saved to: {:?}", mem_file_path);
        Ok(())
    }

    async fn restore(&self, mem_file_path: &Path, state_file_path: &Path) -> Result<()> {
        let load_payload = serde_json::json!({
            "snapshot_path": state_file_path.to_string_lossy(),
            "mem_backend": {
                "backend_path": mem_file_path.to_string_lossy(),
                "backend_type": "File"
            },
            "enable_diff_snapshots": true,
            "resume_vm": true
        });
        self.send_api_request("PUT", "/snapshot/load", Some(load_payload)).await?;
        tracing::info!("Snapshot restored from: {:?}", mem_file_path);
        Ok(())
    }
}
