use shadow_core::Result;
use shadow_vmm::{SwarmOrchestrator, VirtioBalloonConfig, VirtioBalloonDriver};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_virtio_balloon_dynamic_memory_reclamation() -> Result<()> {
    let mut config = VirtioBalloonConfig::default();
    config.initial_memory_mb = 512;
    config.target_idle_mb = 140;

    let mut driver = VirtioBalloonDriver::new(config);
    assert_eq!(driver.current_memory_mb, 512.0);

    // 1. Reclaim idle memory
    let metrics = driver.reclaim_idle_memory()?;
    assert!(metrics.is_idle);
    assert!(metrics.complies_with_idle_target);
    assert_eq!(metrics.current_memory_mb, 140.0);
    assert!(metrics.current_memory_mb < 200.0, "Idle memory must be < 200 MB");

    // 2. Allocate for active workload
    let work_metrics = driver.allocate_workload_memory(256)?;
    assert!(!work_metrics.is_idle);
    assert_eq!(work_metrics.current_memory_mb, 396.0);

    // 3. Reclaim back to idle
    let re_metrics = driver.reclaim_idle_memory()?;
    assert_eq!(re_metrics.current_memory_mb, 140.0);
    assert!(driver.verify_idle_compliance());

    Ok(())
}

#[test]
fn test_swarm_worker_spawn_cpu_pinning_and_memory_quotas() -> Result<()> {
    let temp_root = TempDir::new()?;
    let workspace = temp_root.path().join("swarm_repo");
    fs::create_dir_all(&workspace)?;

    let mut orchestrator = SwarmOrchestrator::new(workspace.clone());

    // Spawn 4 workers with discrete CPU pinning (cores 0..3)
    for i in 0..4 {
        let worker_id = format!("agent-worker-{}", i);
        let info = orchestrator.spawn_worker(&worker_id, Some(i), Some(512))?;

        assert_eq!(info.worker_id, worker_id);
        assert_eq!(info.pinned_cpu, i);
        assert_eq!(info.memory_quota_mb, 512);
        assert!(info.idle_footprint_mb < 200.0, "Worker idle footprint must be < 200 MB");
        assert!(info.spawn_latency_ms < 150.0, "Spawn latency must be < 150ms");

        let upper_path = std::path::PathBuf::from(&info.upper_dir);
        assert!(upper_path.exists());
    }

    assert_eq!(orchestrator.workers.len(), 4);
    assert!(orchestrator.get_aggregate_memory_mb() < 2500.0);

    Ok(())
}

#[test]
fn test_concurrent_multi_agent_execution_and_zero_cross_pollution() -> Result<()> {
    let temp_root = TempDir::new()?;
    let workspace = temp_root.path().join("swarm_concurrency_repo");
    fs::create_dir_all(&workspace)?;
    fs::write(workspace.join("Cargo.toml"), "[package]\nname = 'swarm-test'\n")?;

    let mut orchestrator = SwarmOrchestrator::new(workspace.clone());

    // Spawn 4 Workers
    let w1 = orchestrator.spawn_worker("worker-lint", Some(0), Some(512))?;
    let w2 = orchestrator.spawn_worker("worker-test", Some(1), Some(512))?;
    let w3 = orchestrator.spawn_worker("worker-refactor", Some(2), Some(512))?;
    let w4 = orchestrator.spawn_worker("worker-doc", Some(3), Some(512))?;

    assert_eq!(orchestrator.workers.len(), 4);

    // Dispatch 4 distinct concurrent tasks
    let res1 = orchestrator.dispatch_task("worker-lint", "cargo clippy --workspace", &[])?;
    let res2 = orchestrator.dispatch_task("worker-test", "cargo test --all", &[])?;
    let res3 = orchestrator.dispatch_task("worker-refactor", "refactor backend models", &[])?;
    let res4 = orchestrator.dispatch_task("worker-doc", "generate api doc", &[])?;

    assert_eq!(res1.exit_code, 0);
    assert_eq!(res2.exit_code, 0);
    assert_eq!(res3.exit_code, 0);
    assert_eq!(res4.exit_code, 0);

    // Verify zero cross-agent pollution
    assert!(orchestrator.verify_zero_cross_agent_pollution()?);

    // Verify branching files
    let w3_upper = std::path::PathBuf::from(&w3.upper_dir);
    assert!(w3_upper.join("src").join("models.rs").exists());

    let w1_upper = std::path::PathBuf::from(&w1.upper_dir);
    assert!(!w1_upper.join("src").join("models.rs").exists(), "Worker 1 must not see Worker 3 files!");
    assert!(w1_upper.join("lint_report.txt").exists());

    let w2_upper = std::path::PathBuf::from(&w2.upper_dir);
    assert!(!w2_upper.join("lint_report.txt").exists(), "Worker 2 must not see Worker 1 files!");

    // Collect consolidated swarm results
    let summary = orchestrator.collect_results(None)?;
    assert_eq!(summary.active_workers, 4);
    assert_eq!(summary.total_tasks_completed, 4);
    assert!(summary.aggregate_peak_memory_mb <= 2500.0, "Total memory must remain <= 2.5 GB");
    assert!(summary.within_peak_memory_limit);
    assert!(summary.zero_cross_pollution);

    Ok(())
}
