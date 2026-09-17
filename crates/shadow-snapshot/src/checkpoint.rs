use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use shadow_core::Result;
use shadow_vmm::traits::VMMDriver;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub checkpoint_id: String,
    pub mem_snapshot_path: PathBuf,
    pub state_snapshot_path: PathBuf,
    pub is_diff: bool,
    pub memory_size_bytes: u64,
    pub duration_ms: f64,
}

pub struct CheckpointOrchestrator {
    shm_base_dir: PathBuf,
    checkpoints: HashMap<String, CheckpointMetadata>,
}

impl CheckpointOrchestrator {
    pub fn new(shm_base_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&shm_base_dir);
        Self {
            shm_base_dir,
            checkpoints: HashMap::new(),
        }
    }

    pub fn default_shm() -> Self {
        let base = if Path::new("/dev/shm").exists() {
            PathBuf::from("/dev/shm/shadowos_checkpoints")
        } else {
            std::env::temp_dir().join("shadowos_checkpoints")
        };
        Self::new(base)
    }

    pub fn shm_base_dir(&self) -> &Path {
        &self.shm_base_dir
    }

    pub fn prepare_paths(&self, checkpoint_id: &str) -> (PathBuf, PathBuf) {
        let _ = fs::create_dir_all(&self.shm_base_dir);
        let mem = self.shm_base_dir.join(format!("{}.mem", checkpoint_id));
        let state = self.shm_base_dir.join(format!("{}.state", checkpoint_id));
        (mem, state)
    }

    /// Captures a differential memory and device state snapshot targeting /dev/shm
    pub async fn create_checkpoint<V: VMMDriver>(
        &mut self,
        vmm: &mut V,
        checkpoint_id: &str,
        is_diff: bool,
    ) -> Result<CheckpointMetadata> {
        let start = Instant::now();
        let (mem_path, state_path) = self.prepare_paths(checkpoint_id);

        tracing::info!(
            "Creating {} snapshot '{}' in SHM: {:?}",
            if is_diff { "differential" } else { "full" },
            checkpoint_id,
            self.shm_base_dir
        );

        // Execute hypervisor snapshot
        vmm.snapshot(&mem_path, &state_path).await?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let memory_size_bytes = fs::metadata(&mem_path).map(|m| m.len()).unwrap_or(0);

        let metadata = CheckpointMetadata {
            checkpoint_id: checkpoint_id.to_string(),
            mem_snapshot_path: mem_path,
            state_snapshot_path: state_path,
            is_diff,
            memory_size_bytes,
            duration_ms,
        };

        self.checkpoints.insert(checkpoint_id.to_string(), metadata.clone());
        tracing::info!(
            "Checkpoint '{}' created in {:.2}ms (RAM snapshot: {} bytes)",
            checkpoint_id,
            duration_ms,
            memory_size_bytes
        );

        Ok(metadata)
    }

    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<&CheckpointMetadata> {
        self.checkpoints.get(checkpoint_id)
    }

    pub fn list_checkpoints(&self) -> Vec<CheckpointMetadata> {
        self.checkpoints.values().cloned().collect()
    }

    pub fn delete_checkpoint(&mut self, checkpoint_id: &str) -> Result<bool> {
        if let Some(meta) = self.checkpoints.remove(checkpoint_id) {
            let _ = fs::remove_file(&meta.mem_snapshot_path);
            let _ = fs::remove_file(&meta.state_snapshot_path);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn cleanup_all(&mut self) -> Result<()> {
        for meta in self.checkpoints.values() {
            let _ = fs::remove_file(&meta.mem_snapshot_path);
            let _ = fs::remove_file(&meta.state_snapshot_path);
        }
        self.checkpoints.clear();
        let _ = fs::remove_dir_all(&self.shm_base_dir);
        Ok(())
    }
}
