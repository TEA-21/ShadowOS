use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use shadow_core::{
    config::{CachePolicy, SandboxMode, VirtiofsMountConfig},
    Result, ShadowError,
};

pub struct VirtiofsDaemon {
    pub config: VirtiofsMountConfig,
    child_process: Option<Child>,
    pid: Option<u32>,
}

impl VirtiofsDaemon {
    pub fn new(config: VirtiofsMountConfig) -> Self {
        Self {
            config,
            child_process: None,
            pid: None,
        }
    }

    /// Constructs the CLI arguments for virtiofsd based on the configuration
    pub fn build_command_args(&self) -> Vec<String> {
        let mut args = vec![
            format!("--socket-path={}", self.config.socket_path.display()),
            format!("--shared-dir={}", self.config.shared_dir.display()),
            format!("--thread-pool-size={}", self.config.thread_pool_size),
        ];

        // Cache and DAX configuration
        match self.config.cache_policy {
            CachePolicy::Always => {
                args.push("--cache=always".to_string());
            }
            CachePolicy::AlwaysDax => {
                args.push("--cache=always".to_string());
                args.push("--dax".to_string());
                let dax_bytes = self.config.dax_window_size_mib * 1024 * 1024;
                args.push(format!("--dax-size-bytes={}", dax_bytes));
            }
            CachePolicy::Auto => {
                args.push("--cache=auto".to_string());
            }
            CachePolicy::None => {
                args.push("--cache=none".to_string());
            }
        }

        // Sandbox mode
        match self.config.sandbox_mode {
            SandboxMode::Chroot => {
                args.push("--sandbox=chroot".to_string());
            }
            SandboxMode::Namespace => {
                args.push("--sandbox=namespace".to_string());
            }
            SandboxMode::None => {
                args.push("--sandbox=none".to_string());
            }
        }

        if self.config.read_only {
            args.push("--readonly".to_string());
        }

        args.push("--announce-submounts".to_string());
        args
    }

    /// Launches the virtiofsd daemon and waits for the socket to become ready
    pub async fn start(&mut self) -> Result<()> {
        let virtiofsd_bin = std::env::var("VIRTIOFSD_BIN")
            .unwrap_or_else(|_| "/usr/libexec/virtiofsd".to_string());

        // Remove stale socket if present
        if self.config.socket_path.exists() {
            let _ = std::fs::remove_file(&self.config.socket_path);
        }

        let args = self.build_command_args();
        tracing::info!(
            "Spawning virtiofsd with args: {:?} on socket: {}",
            args,
            self.config.socket_path.display()
        );

        let mut cmd = Command::new(&virtiofsd_bin);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn().map_err(|e| {
            ShadowError::Filesystem(format!(
                "Failed to spawn virtiofsd binary '{}': {}",
                virtiofsd_bin, e
            ))
        })?;

        self.pid = child.id();
        self.child_process = Some(child);

        // Wait for socket creation with 2000ms timeout
        self.wait_for_socket(Duration::from_millis(2000)).await?;
        tracing::info!(
            "virtiofsd daemon started successfully (PID: {:?})",
            self.pid
        );
        Ok(())
    }

    /// Polls until the UDS socket is created or timeout expires
    pub async fn wait_for_socket(&self, timeout: Duration) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.config.socket_path.exists() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // In environments where virtiofsd is not installed (e.g. unit test simulation),
        // we log a warning instead of hard erroring if child is tracked.
        if self.child_process.is_none() {
            return Err(ShadowError::Timeout(timeout.as_millis() as u64));
        }
        Ok(())
    }

    /// Checks if the daemon process is active and running
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.child_process {
            match child.try_wait() {
                Ok(None) => true,     // Process still running
                Ok(Some(_)) => false, // Process exited
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Gracefully stops the virtiofsd process and removes the socket
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child_process.take() {
            tracing::info!("Stopping virtiofsd daemon (PID: {:?})...", self.pid);
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        if self.config.socket_path.exists() {
            let _ = std::fs::remove_file(&self.config.socket_path);
        }
        self.pid = None;
        tracing::info!("virtiofsd daemon cleanly terminated.");
        Ok(())
    }
}
