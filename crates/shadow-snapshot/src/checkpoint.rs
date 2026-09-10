use std::path::{Path, PathBuf};
use std::time::Instant;
use shadow_core::Result;

#[derive(Debug, Clone)]
pub struct CheckpointMetadata {
    pub checkpoint_id: String,
    pub mem_snapshot_path: PathBuf,
    pub state_snapshot_path: PathBuf,
    pub created_at: Instant,
    pub duration_ms: u128,
}

pub struct CheckpointOrchestrator {
    shm_base_dir: PathBuf,
}

impl CheckpointOrchestrator {
    pub fn new(shm_base_dir: PathBuf) -> Self {
        Self { shm_base_dir }
    }

    pub fn default_shm() -> Self {
        let base = if Path::new("/dev/shm").exists() {
            PathBuf::from("/dev/shm/shadowos_checkpoints")
        } else {
            std::env::temp_dir().join("shadowos_checkpoints")
        };
        Self::new(base)
    }

    pub fn prepare_paths(&self, checkpoint_id: &str) -> (PathBuf, PathBuf) {
        let mem = self.shm_base_dir.join(format!("{}.mem", checkpoint_id));
        let state = self.shm_base_dir.join(format!("{}.state", checkpoint_id));
        (mem, state)
    }
}
