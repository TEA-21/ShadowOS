use std::path::Path;
use std::time::Instant;
use shadow_core::Result;
use shadow_cow::OverlayManager;
use shadow_vmm::traits::VMMDriver;

pub struct RollbackController;

impl RollbackController {
    /// Executes the full sub-100ms rollback sequence: pauses VMM, loads RAM snapshot, wipes Overlay upperdir, resumes VMM
    pub async fn execute_rollback<V: VMMDriver>(
        vmm: &mut V,
        overlay_manager: &OverlayManager,
        mem_snapshot_path: &Path,
        state_snapshot_path: &Path,
    ) -> Result<u128> {
        let start = Instant::now();

        // 1. Pause VCPUs (~4ms)
        vmm.pause().await?;

        // 2. Wipe ephemeral OverlayFS upperdir (<5ms)
        overlay_manager.reset_upperdir()?;

        // 3. Restore RAM state from memory-mapped /dev/shm (~45ms)
        vmm.restore(mem_snapshot_path, state_snapshot_path).await?;

        // 4. Resume VCPUs (~3ms)
        vmm.resume().await?;

        let elapsed_ms = start.elapsed().as_millis();
        tracing::info!("Complete state rollback executed in {}ms", elapsed_ms);
        Ok(elapsed_ms)
    }
}
