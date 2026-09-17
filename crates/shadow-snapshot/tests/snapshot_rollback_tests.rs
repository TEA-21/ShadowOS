use shadow_core::Result;
use shadow_cow::{OverlayConfig, OverlayManager};
use shadow_snapshot::{CheckpointOrchestrator, RollbackController};
use shadow_vmm::{MockHypervisor, MockVmState, VMMDriver};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

#[tokio::test]
async fn test_checkpoint_creation_and_pruning() -> Result<()> {
    let temp_shm = TempDir::new()?;
    let mut orchestrator = CheckpointOrchestrator::new(temp_shm.path().to_path_buf());

    let mut vmm = MockHypervisor::new();
    vmm.start().await?;

    // Create full checkpoint
    let cp1 = orchestrator
        .create_checkpoint(&mut vmm, "base_chk_001", false)
        .await?;

    assert_eq!(cp1.checkpoint_id, "base_chk_001");
    assert!(!cp1.is_diff);
    assert!(cp1.mem_snapshot_path.exists());
    assert!(cp1.state_snapshot_path.exists());

    // Create differential checkpoint
    let cp2 = orchestrator
        .create_checkpoint(&mut vmm, "diff_chk_002", true)
        .await?;

    assert_eq!(cp2.checkpoint_id, "diff_chk_002");
    assert!(cp2.is_diff);
    assert_eq!(orchestrator.list_checkpoints().len(), 2);

    // Delete checkpoint
    let deleted = orchestrator.delete_checkpoint("base_chk_001")?;
    assert!(deleted);
    assert!(!cp1.mem_snapshot_path.exists());
    assert!(!cp1.state_snapshot_path.exists());
    assert_eq!(orchestrator.list_checkpoints().len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_sub_100ms_combined_rollback() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");
    let work_dir = temp_root.path().join("workdir");
    let merged_dir = temp_root.path().join("merged_workspace");
    let shm_dir = temp_root.path().join("shm_checkpoints");

    // 1. Setup host repository
    fs::create_dir_all(host_dir.join("src"))?;
    let mut pristine_file = File::create(host_dir.join("src/main.rs"))?;
    pristine_file.write_all(b"fn main() { println!(\"Pristine Host Code\"); }\n")?;

    let initial_hash = OverlayManager::compute_directory_sha256(&host_dir)?;

    // 2. Initialize OverlayFS CoW stack
    let overlay_cfg = OverlayConfig::new(
        host_dir.clone(),
        upper_dir.clone(),
        work_dir.clone(),
        merged_dir.clone(),
    );
    let overlay_manager = OverlayManager::new(overlay_cfg);
    overlay_manager.init_ephemeral_layers()?;

    // 3. Boot VMM and capture baseline checkpoint
    let mut vmm = MockHypervisor::new();
    vmm.start().await?;

    let mut orchestrator = CheckpointOrchestrator::new(shm_dir);
    let checkpoint = orchestrator
        .create_checkpoint(&mut vmm, "golden_state", true)
        .await?;

    // 4. Simulate catastrophic agent pollution in ephemeral upperdir
    fs::create_dir_all(upper_dir.join("src"))?;
    let mut rogue_file = File::create(upper_dir.join("src/rogue_exploit.rs"))?;
    rogue_file.write_all(b"// Malicious agent hallucination\n")?;

    let mut polluted_main = File::create(upper_dir.join("src/main.rs"))?;
    polluted_main.write_all(b"fn main() { panic!(\"Corrupted!\"); }\n")?;

    let mut junk_log = File::create(upper_dir.join("agent_debug.log"))?;
    junk_log.write_all(b"Fatal crash occurred in agent loop\n")?;

    let upper_changes = overlay_manager.scan_upperdir_changes()?;
    assert_eq!(upper_changes.len(), 3);

    // 5. Execute Sub-100ms Combined Rollback (RAM differential + CoW upperdir reset)
    let metrics = RollbackController::execute_rollback(&mut vmm, &overlay_manager, &checkpoint).await?;

    println!(
        "\n[ROLLBACK TEST] Combined Rollback: Total {:.2}ms (Pause: {:.2}ms, CoW Reset: {:.2}ms, RAM Restore: {:.2}ms, Resume: {:.2}ms) - Target (<100ms) Met: {}",
        metrics.total_rollback_duration_ms,
        metrics.pause_duration_ms,
        metrics.overlay_reset_duration_ms,
        metrics.ram_restore_duration_ms,
        metrics.resume_duration_ms,
        metrics.target_met
    );

    // 6. Assertions
    assert!(metrics.target_met, "Rollback duration must be < 100ms");
    assert_eq!(vmm.get_state(), MockVmState::Running);

    // Ephemeral changes must be 100% wiped
    let post_rollback_changes = overlay_manager.scan_upperdir_changes()?;
    assert_eq!(
        post_rollback_changes.len(),
        0,
        "Upperdir must be empty after rollback"
    );

    // Host must remain bit-identical
    let post_rollback_host_hash = OverlayManager::compute_directory_sha256(&host_dir)?;
    assert_eq!(initial_hash, post_rollback_host_hash);

    Ok(())
}

#[tokio::test]
async fn test_50_cycle_sequential_rollback_benchmark() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");
    let work_dir = temp_root.path().join("workdir");
    let merged_dir = temp_root.path().join("merged_workspace");
    let shm_dir = temp_root.path().join("shm_checkpoints");

    fs::create_dir_all(&host_dir)?;
    fs::write(host_dir.join("lib.rs"), b"pub fn code() -> i32 { 42 }\n")?;

    let overlay_cfg = OverlayConfig::new(
        host_dir.clone(),
        upper_dir.clone(),
        work_dir.clone(),
        merged_dir.clone(),
    );
    let overlay_manager = OverlayManager::new(overlay_cfg);
    overlay_manager.init_ephemeral_layers()?;

    let mut vmm = MockHypervisor::new();
    vmm.start().await?;

    let mut orchestrator = CheckpointOrchestrator::new(shm_dir);
    let checkpoint = orchestrator
        .create_checkpoint(&mut vmm, "bench_checkpoint", true)
        .await?;

    let iterations = 50;
    let mut latencies: Vec<f64> = Vec::with_capacity(iterations);

    for i in 0..iterations {
        // Mutate upperdir
        fs::write(
            upper_dir.join(format!("mutation_{:03}.tmp", i)),
            b"ephemeral agent data",
        )?;

        // Rollback
        let metrics =
            RollbackController::execute_rollback(&mut vmm, &overlay_manager, &checkpoint).await?;
        latencies.push(metrics.total_rollback_duration_ms);

        assert_eq!(overlay_manager.scan_upperdir_changes()?.len(), 0);
    }

    let avg_latency = latencies.iter().sum::<f64>() / iterations as f64;
    let min_latency = latencies.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_latency = latencies.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_latency = latencies[(iterations as f64 * 0.95) as usize];

    println!(
        "\n[50-CYCLE BENCHMARK] Iterations: {} -> Avg: {:.2}ms, Min: {:.2}ms, P95: {:.2}ms, Max: {:.2}ms (Target: <100ms)",
        iterations, avg_latency, min_latency, p95_latency, max_latency
    );

    assert!(
        avg_latency < 100.0,
        "Average rollback latency {:.2}ms exceeded 100ms target",
        avg_latency
    );
    assert!(
        p95_latency < 100.0,
        "P95 rollback latency {:.2}ms exceeded 100ms target",
        p95_latency
    );

    Ok(())
}
