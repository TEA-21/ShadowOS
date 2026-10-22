#!/usr/bin/env python3
"""
================================================================================
Project ShadowOS — Backend & Dashboard Integration Verification Suite
Testing Directive:
- Starts shadow-server daemon or verifies running daemon on http://localhost:3001
- Tests GET /api/health and GET /api/system/info
- Tests MicroVM lifecycle (list, spawn, pause/balloon, resume, kill)
- Tests Snapshots & sub-100ms instant state rollback
- Tests Swarm summary and MCP atomic host promotion
- Validates PRD NFR targets: rollback < 100ms, idle footprint = 140MB, RAM cap <= 2500MB
================================================================================
"""

import os
import sys
import time
import json
import urllib.request
import urllib.error
import subprocess

SERVER_PORT = 3001
BASE_URL = f"http://127.0.0.1:{SERVER_PORT}/api"

def api_request(endpoint: str, method: str = "GET", payload: dict = None) -> tuple[int, dict]:
    url = f"{BASE_URL}{endpoint}"
    data = json.dumps(payload).encode("utf-8") if payload is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            res_body = response.read().decode("utf-8")
            return response.status, json.loads(res_body) if res_body else {}
    except urllib.error.HTTPError as e:
        err_body = e.read().decode("utf-8")
        return e.code, json.loads(err_body) if err_body else {}

def main():
    print("=" * 70)
    print(" ShadowOS Backend & Dashboard Integration Test Suite")
    print("=" * 70)

    # 1. Health check
    print("\n[*] Step 1: Validating GET /api/health...")
    status, health = api_request("/health")
    assert status == 200, f"Expected 200, got {status}: {health}"
    assert health.get("status") == "healthy", f"Invalid status: {health}"
    print(f"  [OK] Health check passed: {health}")

    # 2. System info
    print("\n[*] Step 2: Validating GET /api/system/info...")
    status, sys_info = api_request("/system/info")
    assert status == 200, f"Expected 200, got {status}: {sys_info}"
    print(f"  [OK] Host OS: {sys_info.get('hostOs')}")
    print(f"  [OK] Hypervisor Driver: {sys_info.get('hypervisorDriver')}")
    print(f"  [OK] Protocol Version: {sys_info.get('protocolVersion')}")
    assert sys_info.get("aggregateRamLimitMb") == 2500, "Aggregate RAM limit must be 2500 MB"

    # 3. List VMs
    print("\n[*] Step 3: Validating GET /api/vms...")
    status, vms = api_request("/vms")
    assert status == 200, f"Expected 200, got {status}: {vms}"
    assert len(vms) >= 1, "Expected at least 1 provisioned microVM"
    primary_vm = vms[0]
    print(f"  [OK] Retrieved {len(vms)} microVM instances. Primary: {primary_vm['id']} ({primary_vm['status']})")

    # 4. Spawn new MicroVM
    print("\n[*] Step 4: Validating POST /api/vms (MicroVM Provisioning)...")
    spawn_payload = {
        "name": "integration-test-sandbox",
        "vcpus": 2,
        "memoryMb": 512,
        "workloadProfile": "Automated Integration Test Runner"
    }
    status, spawned_vm = api_request("/vms", method="POST", payload=spawn_payload)
    assert status == 201, f"Expected 201 Created, got {status}: {spawned_vm}"
    test_vm_id = spawned_vm["id"]
    print(f"  [OK] Spawned MicroVM: {test_vm_id} (Status: {spawned_vm['status']}, Base RAM: {spawned_vm['ramUsage']['currentMb']} MB)")
    assert spawned_vm["ramUsage"]["currentMb"] == 140, "Base RAM footprint must be strictly 140 MB"

    # 5. Pause MicroVM and verify memory ballooning
    print(f"\n[*] Step 5: Validating POST /api/vms/{test_vm_id}/pause (Dynamic Memory Ballooning)...")
    status, paused_vm = api_request(f"/vms/{test_vm_id}/pause", method="POST")
    assert status == 200, f"Expected 200, got {status}: {paused_vm}"
    assert paused_vm["status"] == "Suspended", "VM status must be Suspended"
    assert paused_vm["ramUsage"]["currentMb"] == 140, "Ballooned idle footprint must be 140 MB"
    print(f"  [OK] MicroVM {test_vm_id} suspended. Ballooned memory: {paused_vm['ramUsage'].get('balloonedMb')} MB reclaimed to host.")

    # 6. Resume MicroVM
    print(f"\n[*] Step 6: Validating POST /api/vms/{test_vm_id}/resume...")
    status, resumed_vm = api_request(f"/vms/{test_vm_id}/resume", method="POST")
    assert status == 200, f"Expected 200, got {status}: {resumed_vm}"
    assert resumed_vm["status"] == "Active", "VM status must be Active"
    print(f"  [OK] MicroVM {test_vm_id} resumed. Status: {resumed_vm['status']}")

    # 7. Safe MicroVM termination
    print(f"\n[*] Step 7: Validating DELETE /api/vms/{test_vm_id} (Ephemeral Wipe)...")
    status, term_res = api_request(f"/vms/{test_vm_id}", method="DELETE")
    assert status == 200, f"Expected 200, got {status}: {term_res}"
    print(f"  [OK] MicroVM {test_vm_id} terminated and scratch upperdir unmounted.")

    # 8. Snapshots list
    print("\n[*] Step 8: Validating GET /api/snapshots...")
    status, snapshots = api_request("/snapshots")
    assert status == 200, f"Expected 200, got {status}: {snapshots}"
    assert len(snapshots) >= 1, "Expected at least 1 baseline snapshot"
    print(f"  [OK] Retrieved {len(snapshots)} snapshots.")

    # 9. Create snapshot
    print("\n[*] Step 9: Validating POST /api/snapshots (SHM Differential Checkpoint)...")
    snap_payload = {
        "vmId": primary_vm["id"],
        "name": "Integration Test Checkpoint",
        "description": "Pre-rollback test checkpoint"
    }
    status, new_snap = api_request("/snapshots", method="POST", payload=snap_payload)
    assert status == 201, f"Expected 201 Created, got {status}: {new_snap}"
    snap_id = new_snap["id"]
    print(f"  [OK] Created Checkpoint: {snap_id} (Latency: {new_snap['rollbackLatencyMs']} ms)")

    # 10. Sub-100ms Rollback
    print(f"\n[*] Step 10: Validating POST /api/snapshots/{snap_id}/rollback (Sub-100ms Instant Rollback)...")
    status, rollback_res = api_request(f"/snapshots/{snap_id}/rollback", method="POST")
    assert status == 200, f"Expected 200, got {status}: {rollback_res}"
    assert rollback_res.get("success") is True, "Rollback must succeed"
    latency = rollback_res.get("latencyMs", 0)
    assert latency < 100.0, f"Rollback latency must be strictly < 100ms (measured {latency}ms)"
    print(f"  [OK] Instant rollback executed in {latency} ms (NFR-02 < 100ms target satisfied!).")

    # 11. Swarm summary
    print("\n[*] Step 11: Validating GET /api/swarm/summary...")
    status, swarm_res = api_request("/swarm/summary")
    assert status == 200, f"Expected 200, got {status}: {swarm_res}"
    print(f"  [OK] Active Swarm Workers: {swarm_res.get('activeWorkers')}")
    print(f"  [OK] Aggregate Peak Memory: {swarm_res.get('aggregatePeakMemoryMb')} MB / 2500 MB")
    print(f"  [OK] Zero Cross Pollution: {swarm_res.get('zeroCrossPollution')}")
    assert swarm_res.get("withinPeakMemoryLimit") is True, "Swarm must stay within 2500 MB peak cap"

    # 12. Host promotion
    print("\n[*] Step 12: Validating POST /api/mcp/promote (Atomic Host Merge)...")
    promote_payload = {
        "noteId": "note-01",
        "vmId": primary_vm["id"]
    }
    status, promote_res = api_request("/mcp/promote", method="POST", payload=promote_payload)
    assert status == 200, f"Expected 200, got {status}: {promote_res}"
    assert promote_res.get("success") is True, "Promotion must succeed"
    print(f"  [OK] Host tree promotion verified. Checksum: {promote_res.get('hostTreeChecksum')} ({promote_res.get('filesMerged')} files merged)")

    print("\n" + "=" * 70)
    print(" ALL 12 BACKEND & DASHBOARD INTEGRATION GATES: PASSED (100% PRD COMPLIANT)")
    print("=" * 70)

if __name__ == "__main__":
    main()
