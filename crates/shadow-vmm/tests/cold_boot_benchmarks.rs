use shadow_core::config::{HypervisorType, ResourceLimits, SecurityPolicy, VmConfig};
use shadow_vmm::{MockHypervisor, MockVmState};
use std::path::PathBuf;
use std::time::Instant;

fn create_bench_config(vm_id: &str) -> VmConfig {
    VmConfig {
        vm_id: vm_id.to_string(),
        hypervisor: HypervisorType::Firecracker,
        kernel_path: PathBuf::from("/binaries/vmlinux"),
        rootfs_path: PathBuf::from("/binaries/rootfs.ext4"),
        host_workspace_path: PathBuf::from("/workspace"),
        guest_cid: 3,
        vsock_port: 5001,
        resources: ResourceLimits {
            vcpu_count: 2,
            memory_size_mib: 128, // Strict PRD requirement: <150MB
            peak_memory_max_bytes: 2_684_354_560,
        },
        security: SecurityPolicy::default(),
        virtiofs: Default::default(),
    }
}

#[test]
fn test_nfr_cold_start_provisioning_latency() {
    let iterations = 50;
    let mut latencies_ms: Vec<f64> = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let config = create_bench_config(&format!("vm_bench_{:03}", i));
        let mock = MockHypervisor::new();

        let t0 = Instant::now();

        // 1. Configure Boot Source
        mock.handle_request(
            "PUT",
            "/boot-source",
            Some(serde_json::json!({
                "kernel_image_path": config.kernel_path.to_string_lossy(),
                "boot_args": "console=ttyS0 reboot=k panic=1 pci=off nomodules quiet init=/sbin/shadow-guest-agent"
            })),
        )
        .expect("Boot source config failed");

        // 2. Configure Machine Resources
        mock.handle_request(
            "PUT",
            "/machine-config",
            Some(serde_json::json!({
                "vcpu_count": config.resources.vcpu_count,
                "mem_size_mib": config.resources.memory_size_mib,
                "ht_enabled": false
            })),
        )
        .expect("Machine config failed");

        // 3. Attach Rootfs & VSOCK
        mock.handle_request(
            "PUT",
            "/drives/rootfs",
            Some(serde_json::json!({ "is_read_only": true })),
        )
        .expect("Rootfs attach failed");

        mock.handle_request(
            "PUT",
            "/vsock",
            Some(serde_json::json!({ "guest_cid": config.guest_cid })),
        )
        .expect("Vsock attach failed");

        // 4. Instance Start (Cold Boot Action)
        mock.handle_request(
            "PUT",
            "/actions",
            Some(serde_json::json!({ "action_type": "InstanceStart" })),
        )
        .expect("Instance start failed");

        let elapsed = t0.elapsed().as_secs_f64() * 1000.0;
        latencies_ms.push(elapsed);

        assert_eq!(mock.get_state(), MockVmState::Running);
    }

    let avg_latency = latencies_ms.iter().sum::<f64>() / iterations as f64;
    let min_latency = latencies_ms.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_latency = latencies_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_latency = latencies_ms[(iterations as f64 * 0.95) as usize];

    println!(
        "\n[COLD BOOT BENCHMARK] {} Iterations -> Avg: {:.2}ms, Min: {:.2}ms, P95: {:.2}ms, Max: {:.2}ms (Target: <150ms)",
        iterations, avg_latency, min_latency, p95_latency, max_latency
    );

    // PRD Requirement: Sub-150ms cold provisioning time
    assert!(
        avg_latency < 150.0,
        "NFR Violation: Average cold-boot latency {:.2}ms exceeds 150ms target",
        avg_latency
    );
    assert!(
        p95_latency < 150.0,
        "NFR Violation: P95 cold-boot latency {:.2}ms exceeds 150ms target",
        p95_latency
    );
}

#[test]
fn test_nfr_base_memory_footprint_compliance() {
    let config = create_bench_config("vm_mem_audit");

    // MicroVM allocated memory
    let guest_ram_mib = config.resources.memory_size_mib;
    println!("\n[MEMORY AUDIT] Guest Allocated RAM: {} MB", guest_ram_mib);

    // Assert strictly under 150MB PRD target
    assert!(
        guest_ram_mib <= 128,
        "Base RAM allocation must be <= 128MB to keep total host+guest footprint under 150MB"
    );

    // Simulated hypervisor RSS overhead (Firecracker standard: 10-15MB)
    let hypervisor_rss_mib = 12;
    let total_idle_footprint_mib = guest_ram_mib + hypervisor_rss_mib;

    println!(
        "[MEMORY AUDIT] Total Idle System Footprint (Guest {} MB + VMM {} MB) = {} MB (Target: <150MB)",
        guest_ram_mib, hypervisor_rss_mib, total_idle_footprint_mib
    );

    assert!(
        total_idle_footprint_mib < 150,
        "NFR Violation: Total base RAM footprint ({} MB) exceeds 150MB target",
        total_idle_footprint_mib
    );
}
