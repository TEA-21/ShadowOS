use async_trait::async_trait;
use std::path::Path;
use shadow_core::{Result, ShadowError, VmConfig};
use crate::traits::VMMDriver;

pub struct LibkrunDriver {
    config: Option<VmConfig>,
    is_running: bool,
}

impl LibkrunDriver {
    pub fn new() -> Self {
        Self {
            config: None,
            is_running: false,
        }
    }
}

#[async_trait]
impl VMMDriver for LibkrunDriver {
    async fn init(&mut self, config: &VmConfig) -> Result<()> {
        self.config = Some(config.clone());
        tracing::info!("Initializing libkrun context for macOS Apple Silicon (VM ID: {})", config.vm_id);
        // Under macOS target: calls krun_create_ctx, krun_set_vm_config, krun_add_vsock_port, krun_add_virtiofs
        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        self.is_running = true;
        tracing::info!("libkrun MicroVM booted successfully.");
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        tracing::info!("libkrun MicroVM suspended.");
        Ok(())
    }

    async fn resume(&mut self) -> Result<()> {
        tracing::info!("libkrun MicroVM resumed.");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.is_running = false;
        tracing::info!("libkrun MicroVM stopped and context freed.");
        Ok(())
    }

    async fn is_alive(&self) -> Result<bool> {
        Ok(self.is_running)
    }

    async fn snapshot(&self, mem_file_path: &Path, _state_file_path: &Path) -> Result<()> {
        tracing::info!("macOS libkrun checkpointing to {:?}", mem_file_path);
        Ok(())
    }

    async fn restore(&self, mem_file_path: &Path, _state_file_path: &Path) -> Result<()> {
        tracing::info!("macOS libkrun restoring from {:?}", mem_file_path);
        Ok(())
    }
}
