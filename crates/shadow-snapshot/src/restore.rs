use std::path::Path;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use shadow_core::Result;
use shadow_cow::OverlayManager;
use shadow_vmm::traits::VMMDriver;
use crate::checkpoint::CheckpointMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackMetrics {
    pub checkpoint_id: String,
    pub pause_duration_ms: f64,
    pub overlay_reset_duration_ms: f64,
    pub ram_restore_duration_ms: f64,
    pub resume_duration_ms: f64,
    pub total_rollback_duration_ms: f64,
    pub target_met: bool,
}

pub struct RollbackController;

impl RollbackController {
    /// Executes the complete sub-100ms rollback sequence combining RAM restore and OverlayFS wipe.
    pub async fn execute_rollback<V: VMMDriver>(
        vmm: &mut V,
        overlay_manager: &OverlayManager,
        checkpoint: &CheckpointMetadata,
    ) -> Result<RollbackMetrics> {
        Self::execute_rollback_paths(
            vmm,
            overlay_manager,
            &checkpoint.checkpoint_id,
            &checkpoint.mem_snapshot_path,
            &checkpoint.state_snapshot_path,
        )
        .await
    }

    /// Executes rollback using direct snapshot paths with full microsecond-precision breakdown
    pub async fn execute_rollback_paths<V: VMMDriver>(
        vmm: &mut V,
        overlay_manager: &OverlayManager,
        checkpoint_id: &str,
        mem_snapshot_path: &Path,
        state_snapshot_path: &Path,
    ) -> Result<RollbackMetrics> {
        let total_start = Instant::now();

        // 1. Pause VCPUs (~2-5ms)
        let t_pause = Instant::now();
        vmm.pause().await?;
        let pause_duration_ms = t_pause.elapsed().as_secs_f64() * 1000.0;

        // 2. Wipe ephemeral OverlayFS upperdir & workdir (<5ms, zero host pollution)
        let t_overlay = Instant::now();
        overlay_manager.reset_upperdir()?;
        let overlay_reset_duration_ms = t_overlay.elapsed().as_secs_f64() * 1000.0;

        // 3. Restore RAM state from memory-mapped /dev/shm differential snapshot (~20-45ms)
        let t_restore = Instant::now();
        vmm.restore(mem_snapshot_path, state_snapshot_path).await?;
        let ram_restore_duration_ms = t_restore.elapsed().as_secs_f64() * 1000.0;

        // 4. Resume VCPUs (~2-3ms)
        let t_resume = Instant::now();
        vmm.resume().await?;
        let resume_duration_ms = t_resume.elapsed().as_secs_f64() * 1000.0;

        let total_rollback_duration_ms = total_start.elapsed().as_secs_f64() * 1000.0;
        let target_met = total_rollback_duration_ms < 100.0;

        let metrics = RollbackMetrics {
            checkpoint_id: checkpoint_id.to_string(),
            pause_duration_ms,
            overlay_reset_duration_ms,
            ram_restore_duration_ms,
            resume_duration_ms,
            total_rollback_duration_ms,
            target_met,
        };

        tracing::info!(
            "[ROLLBACK] Checkpoint '{}' restored in {:.2}ms (Pause: {:.2}ms, CoW Reset: {:.2}ms, RAM Restore: {:.2}ms, Resume: {:.2}ms) - Target (<100ms) Met: {}",
            checkpoint_id,
            total_rollback_duration_ms,
            pause_duration_ms,
            overlay_reset_duration_ms,
            ram_restore_duration_ms,
            resume_duration_ms,
            target_met
        );

        Ok(metrics)
    }
}
