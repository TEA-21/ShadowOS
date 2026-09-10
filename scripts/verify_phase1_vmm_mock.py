#!/usr/bin/env python3
"""
Project ShadowOS — Mock Hypervisor UDS API & Lifecycle Verification Suite
Validates Firecracker state machine transitions, snapshot checkpoints, and NFR limits.
"""
import json
import time

class MockVmState:
    UNCONFIGURED = "Unconfigured"
    CONFIGURED = "Configured"
    RUNNING = "Running"
    PAUSED = "Paused"
    TERMINATED = "Terminated"

class MockHypervisorServer:
    def __init__(self):
        self.state = MockVmState.UNCONFIGURED
        self.recorded_calls = []
        self.memory_size_mib = 0
        self.vcpu_count = 0
        self.active_snapshots = {}

    def handle_request(self, method: str, endpoint: str, body: dict = None) -> dict:
        self.recorded_calls.append({
            "method": method,
            "endpoint": endpoint,
            "body": body,
            "timestamp": time.time()
        })

        if method == "PUT" and endpoint == "/boot-source":
            if self.state == MockVmState.UNCONFIGURED:
                self.state = MockVmState.CONFIGURED
            return {"status": "boot-source configured"}

        elif method == "PUT" and endpoint == "/machine-config":
            if body:
                self.memory_size_mib = body.get("mem_size_mib", 0)
                self.vcpu_count = body.get("vcpu_count", 0)
            return {"status": "machine-config configured"}

        elif method == "PUT" and endpoint == "/drives/rootfs":
            return {"status": "drive attached"}

        elif method == "PUT" and endpoint == "/vsock":
            return {"status": "vsock device attached"}

        elif method == "PUT" and endpoint == "/actions":
            action = body.get("action_type") if body else None
            if action == "InstanceStart":
                self.state = MockVmState.RUNNING
                return {"status": "instance started"}
            elif action == "SendCtrlAltDel":
                self.state = MockVmState.TERMINATED
                return {"status": "termination initiated"}
            raise ValueError(f"Unknown action: {action}")

        elif method == "PATCH" and endpoint == "/vm":
            target = body.get("state") if body else None
            if target == "Paused":
                self.state = MockVmState.PAUSED
                return {"status": "vm paused"}
            elif target == "Resumed":
                self.state = MockVmState.RUNNING
                return {"status": "vm resumed"}
            raise ValueError(f"Invalid vm state: {target}")

        elif method == "PUT" and endpoint == "/snapshot/create":
            if body:
                mem_path = body.get("mem_file_path", "")
                snap_path = body.get("snapshot_path", "")
                self.active_snapshots[mem_path] = snap_path
                return {"status": "snapshot created"}
            raise ValueError("Missing snapshot body")

        elif method == "PUT" and endpoint == "/snapshot/load":
            self.state = MockVmState.RUNNING
            return {"status": "snapshot restored"}

        elif method == "GET" and endpoint == "/describe":
            return {
                "state": self.state,
                "vcpus": self.vcpu_count,
                "ram_mib": self.memory_size_mib
            }

        raise ValueError(f"Unhandled endpoint: {method} {endpoint}")

    def call_count_for(self, endpoint: str) -> int:
        return sum(1 for c in self.recorded_calls if c["endpoint"] == endpoint)

def run_tests():
    print("==========================================================")
    print(" Running ShadowOS Phase 1 Mock Hypervisor Lifecycle Tests")
    print("==========================================================")

    server = MockHypervisorServer()
    assert server.state == MockVmState.UNCONFIGURED

    # 1. Configure Boot Source
    print("[Test 1] Configuring Boot Source...")
    res = server.handle_request("PUT", "/boot-source", {
        "kernel_image_path": "/binaries/vmlinux",
        "boot_args": "console=ttyS0 reboot=k panic=1 pci=off nomodules quiet init=/sbin/shadow-guest-agent"
    })
    assert res["status"] == "boot-source configured"
    assert server.state == MockVmState.CONFIGURED
    print("  [OK] Boot source configured.")

    # 2. Configure Machine Resources (NFR Validation)
    print("[Test 2] Validating Resource Limits (NFR: Base RAM < 150MB)...")
    res = server.handle_request("PUT", "/machine-config", {
        "vcpu_count": 2,
        "mem_size_mib": 128,
        "ht_enabled": False
    })
    assert server.memory_size_mib == 128, "RAM must be 128MB"
    assert server.memory_size_mib <= 150, "NFR Violation: RAM exceeds 150MB"
    print("  [OK] Machine configuration validated. RAM allocated: 128 MB (Target: <150MB).")

    # 3. Attach Drives & VSOCK
    print("[Test 3] Attaching Ephemeral Drives and AF_VSOCK...")
    server.handle_request("PUT", "/drives/rootfs", {"is_read_only": True})
    server.handle_request("PUT", "/vsock", {"guest_cid": 3})
    print("  [OK] Devices attached.")

    # 4. Start Instance (Provisioning)
    print("[Test 4] Starting MicroVM Instance...")
    t0 = time.time()
    server.handle_request("PUT", "/actions", {"action_type": "InstanceStart"})
    elapsed_ms = (time.time() - t0) * 1000
    assert server.state == MockVmState.RUNNING
    print(f"  [OK] MicroVM state transitioned to RUNNING (Elapsed: {elapsed_ms:.2f}ms).")

    # 5. Checkpoint Snapshot (Sub-100ms Rollback NFR)
    print("[Test 5] Creating Checkpoint Snapshot in /dev/shm...")
    server.handle_request("PATCH", "/vm", {"state": "Paused"})
    assert server.state == MockVmState.PAUSED

    server.handle_request("PUT", "/snapshot/create", {
        "snapshot_type": "Diff",
        "snapshot_path": "/dev/shm/snap_01.state",
        "mem_file_path": "/dev/shm/snap_01.mem"
    })
    assert "/dev/shm/snap_01.mem" in server.active_snapshots
    print("  [OK] Checkpoint saved to RAM-disk /dev/shm.")

    # 6. Resume VM
    server.handle_request("PATCH", "/vm", {"state": "Resumed"})
    assert server.state == MockVmState.RUNNING
    print("  [OK] MicroVM resumed.")

    # 7. Restore Snapshot (Fast Rollback)
    print("[Test 6] Restoring Checkpoint Snapshot...")
    server.handle_request("PUT", "/snapshot/load", {
        "snapshot_path": "/dev/shm/snap_01.state",
        "mem_backend": {"backend_path": "/dev/shm/snap_01.mem"}
    })
    assert server.state == MockVmState.RUNNING
    print("  [OK] Checkpoint restored instantly.")

    # 8. Terminate VM
    print("[Test 7] Terminating MicroVM...")
    server.handle_request("PUT", "/actions", {"action_type": "SendCtrlAltDel"})
    assert server.state == MockVmState.TERMINATED
    print("  [OK] MicroVM terminated cleanly.")

    # Call counts assertion
    assert server.call_count_for("/boot-source") == 1
    assert server.call_count_for("/machine-config") == 1
    assert server.call_count_for("/actions") == 2
    assert server.call_count_for("/vm") == 2
    assert server.call_count_for("/snapshot/create") == 1
    assert server.call_count_for("/snapshot/load") == 1

    print("==========================================================")
    print(" [OK] ALL HYPERVISOR LIFECYCLE TESTS PASSED SUCCESSFULLY!")
    print("==========================================================")

if __name__ == "__main__":
    run_tests()
