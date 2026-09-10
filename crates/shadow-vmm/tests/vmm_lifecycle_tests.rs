use shadow_core::{
    config::{HypervisorType, ResourceLimits, SecurityPolicy, VmConfig},
    error::Result,
};
use shadow_vmm::{MockHypervisor, MockVmState};
use std::path::PathBuf;

fn create_test_config() -> VmConfig {
    VmConfig {
        vm_id: "vm_test_01".to_string(),
        hypervisor: HypervisorType::Firecracker,
        kernel_path: PathBuf::from("/binaries/vmlinux"),
        rootfs_path: PathBuf::from("/binaries/rootfs.ext4"),
        host_workspace_path: PathBuf::from("/host/workspace"),
        guest_cid: 3,
        vsock_port: 5001,
        resources: ResourceLimits {
            vcpu_count: 2,
            memory_size_mib: 128,
            peak_memory_max_bytes: 2_684_354_560,
        },
        security: SecurityPolicy::default(),
    }
}

#[test]
fn test_mock_hypervisor_lifecycle_transitions() -> Result<()> {
    let mock = MockHypervisor::new();
    assert_eq!(mock.get_state(), MockVmState::Unconfigured);

    // 1. Configure Boot Source
    let boot_payload = serde_json::json!({
        "kernel_image_path": "/binaries/vmlinux",
        "boot_args": "console=ttyS0 reboot=k panic=1 pci=off nomodules quiet init=/sbin/shadow-guest-agent"
    });
    mock.handle_request("PUT", "/boot-source", Some(boot_payload))?;
    assert_eq!(mock.get_state(), MockVmState::Configured);

    // 2. Configure Machine Resources (NFR: 128MB RAM)
    let machine_payload = serde_json::json!({
        "vcpu_count": 2,
        "mem_size_mib": 128,
        "ht_enabled": false
    });
    mock.handle_request("PUT", "/machine-config", Some(machine_payload))?;
    assert_eq!(*mock.memory_size_mib.lock().unwrap(), 128);
    assert_eq!(*mock.vcpu_count.lock().unwrap(), 2);

    // 3. Attach Rootfs and VSOCK
    mock.handle_request("PUT", "/drives/rootfs", Some(serde_json::json!({ "is_read_only": true })))?;
    mock.handle_request("PUT", "/vsock", Some(serde_json::json!({ "guest_cid": 3 })))?;

    // 4. Start Instance (Provisioning)
    mock.handle_request("PUT", "/actions", Some(serde_json::json!({ "action_type": "InstanceStart" })))?;
    assert_eq!(mock.get_state(), MockVmState::Running);

    // Query status
    let desc = mock.handle_request("GET", "/describe", None)?;
    assert_eq!(desc["state"], "Running");

    // 5. Pause VM (Prepare for Rollback / Snapshot)
    mock.handle_request("PATCH", "/vm", Some(serde_json::json!({ "state": "Paused" })))?;
    assert_eq!(mock.get_state(), MockVmState::Paused);

    // 6. Create Snapshot to /dev/shm (Sub-100ms Rollback NFR)
    let snap_payload = serde_json::json!({
        "snapshot_type": "Diff",
        "snapshot_path": "/dev/shm/snap_vm01.state",
        "mem_file_path": "/dev/shm/snap_vm01.mem",
    });
    mock.handle_request("PUT", "/snapshot/create", Some(snap_payload))?;
    assert!(mock.active_snapshots.lock().unwrap().contains_key("/dev/shm/snap_vm01.mem"));

    // 7. Resume VM
    mock.handle_request("PATCH", "/vm", Some(serde_json::json!({ "state": "Resumed" })))?;
    assert_eq!(mock.get_state(), MockVmState::Running);

    // 8. Restore Snapshot (Fast Rollback Checkpoint)
    let restore_payload = serde_json::json!({
        "snapshot_path": "/dev/shm/snap_vm01.state",
        "mem_backend": { "backend_path": "/dev/shm/snap_vm01.mem" }
    });
    mock.handle_request("PUT", "/snapshot/load", Some(restore_payload))?;
    assert_eq!(mock.get_state(), MockVmState::Running);

    // 9. Stop / Terminate VM
    mock.handle_request("PUT", "/actions", Some(serde_json::json!({ "action_type": "SendCtrlAltDel" })))?;
    assert_eq!(mock.get_state(), MockVmState::Terminated);

    // Verify recorded call traces
    assert_eq!(mock.call_count_for("/boot-source"), 1);
    assert_eq!(mock.call_count_for("/machine-config"), 1);
    assert_eq!(mock.call_count_for("/actions"), 2);
    assert_eq!(mock.call_count_for("/vm"), 2);
    assert_eq!(mock.call_count_for("/snapshot/create"), 1);
    assert_eq!(mock.call_count_for("/snapshot/load"), 1);

    Ok(())
}

#[test]
fn test_nfr_memory_limit_verification() {
    let config = create_test_config();
    // PRD NFR: Base RAM under 150MB
    assert!(
        config.resources.memory_size_mib <= 150,
        "Base allocated RAM must not exceed 150MB, got {}",
        config.resources.memory_size_mib
    );
    // PRD NFR: Peak RAM capped at 2.5GB (2,684,354,560 bytes)
    assert!(
        config.resources.peak_memory_max_bytes <= 2_684_354_560,
        "Peak workload RAM must be capped at 2.5GB"
    );
}
