use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};
use shadow_core::{Result, ShadowError};

pub struct VirtiofsDaemon {
    shared_dir: PathBuf,
    socket_path: PathBuf,
    child_process: Option<Child>,
    read_only: bool,
}

impl VirtiofsDaemon {
    pub fn new(shared_dir: PathBuf, socket_path: PathBuf, read_only: bool) -> Self {
        Self {
            shared_dir,
            socket_path,
            child_process: None,
            read_only,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        let virtiofsd_bin = std::env::var("VIRTIOFSD_BIN")
            .unwrap_or_else(|_| "/usr/libexec/virtiofsd".to_string());

        let mut cmd = Command::new(virtiofsd_bin);
        cmd.arg(format!("--socket-path={}", self.socket_path.display()))
            .arg(format!("--shared-dir={}", self.shared_dir.display()))
            .arg("--cache=always")
            .arg("--sandbox=chroot")
            .arg("--thread-pool-size=4")
            .arg("--announce-submounts");

        if self.read_only {
            cmd.arg("--readonly");
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tracing::info!(
            "Starting virtiofsd on socket: {} for dir: {}",
            self.socket_path.display(),
            self.shared_dir.display()
        );

        let child = cmd.spawn().map_err(|e| {
            ShadowError::Filesystem(format!("Failed to spawn virtiofsd: {}", e))
        })?;

        self.child_process = Some(child);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            let _ = child.kill().await;
            tracing::info!("virtiofsd process terminated.");
        }
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        Ok(())
    }
}
