use shadow_core::config::{CachePolicy, SandboxMode, VirtiofsMountConfig};
use shadow_vmm::{DaxIoBenchmark, VirtiofsDaemon};
use std::path::PathBuf;

#[test]
fn test_virtiofsd_cli_argument_generation() {
    let config = VirtiofsMountConfig {
        tag: "shadow-workspace".to_string(),
        socket_path: PathBuf::from("/run/shadow_virtiofs.sock"),
        shared_dir: PathBuf::from("/home/user/project"),
        cache_policy: CachePolicy::AlwaysDax,
        sandbox_mode: SandboxMode::Chroot,
        thread_pool_size: 4,
        read_only: true,
        dax_window_size_mib: 1024,
    };

    let daemon = VirtiofsDaemon::new(config);
    let args = daemon.build_command_args();

    assert!(args.contains(&"--socket-path=/run/shadow_virtiofs.sock".to_string()));
    assert!(args.contains(&"--shared-dir=/home/user/project".to_string()));
    assert!(args.contains(&"--cache=always".to_string()));
    assert!(args.contains(&"--dax".to_string()));
    assert!(args.contains(&"--dax-size-bytes=1073741824".to_string())); // 1GB in bytes
    assert!(args.contains(&"--sandbox=chroot".to_string()));
    assert!(args.contains(&"--thread-pool-size=4".to_string()));
    assert!(args.contains(&"--readonly".to_string()));
    assert!(args.contains(&"--announce-submounts".to_string()));
}

#[test]
fn test_virtiofsd_writable_and_cache_modes() {
    let mut config = VirtiofsMountConfig::default();
    config.read_only = false;
    config.cache_policy = CachePolicy::Auto;
    config.sandbox_mode = SandboxMode::Namespace;

    let daemon = VirtiofsDaemon::new(config);
    let args = daemon.build_command_args();

    assert!(!args.contains(&"--readonly".to_string()));
    assert!(args.contains(&"--cache=auto".to_string()));
    assert!(args.contains(&"--sandbox=namespace".to_string()));
    assert!(!args.contains(&"--dax".to_string()));
}

#[test]
fn test_dax_io_benchmark_nfr_target_compliance() {
    let temp_dir = std::env::temp_dir().join("shadowos_io_test");
    let test_file_size = 8 * 1024 * 1024; // 8MB test block

    let benchmark = DaxIoBenchmark::new(temp_dir.clone(), test_file_size);
    let results = benchmark.run_evaluation().expect("I/O benchmark should succeed");

    assert_eq!(results.len(), 2);
    for res in results {
        println!(
            "\n[I/O BENCHMARK] {}: Native={:.1} MB/s, VirtIO-FS DAX={:.1} MB/s, Ratio={:.2}% (RAM-disk={:.1} MB/s)",
            res.operation,
            res.native_throughput_mb_s,
            res.virtiofs_dax_throughput_mb_s,
            res.ratio_percentage,
            res.ramdisk_throughput_mb_s
        );

        // PRD NFR: virtio-fs must achieve at least 85% of native throughput
        assert!(
            res.ratio_percentage >= 85.0,
            "NFR VIOLATION: {} achieved only {:.2}% of native NVMe throughput (Target: >=85%)",
            res.operation,
            res.ratio_percentage
        );

        // PRD NFR: In-memory builds (/tmp RAM-disk) must exceed host physical disk speeds
        assert!(
            res.ramdisk_throughput_mb_s >= res.native_throughput_mb_s,
            "NFR VIOLATION: RAM-disk speed ({:.1} MB/s) failed to exceed native disk speed ({:.1} MB/s)",
            res.ramdisk_throughput_mb_s,
            res.native_throughput_mb_s
        );
    }

    let _ = std::fs::remove_dir_all(temp_dir);
}
