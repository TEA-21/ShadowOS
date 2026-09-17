#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Master Test & Benchmark Suite Runner
# ==============================================================================
set -euo pipefail

echo "=========================================================="
echo " Running ShadowOS Test Suites & NFR Benchmarks"
echo "=========================================================="

echo "[1/8] Running shadow-core unit & protocol integration tests..."
cargo test -p shadow-core --test protocol_test -- --nocapture

echo "[2/8] Running shadow-core NFR framing benchmarks..."
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

echo "[3/8] Running shadow-vmm lifecycle state tests..."
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

echo "[4/8] Running shadow-vsock channel queue tests..."
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

echo "[5/8] Running shadow-vsock streaming bash execution tests..."
cargo test -p shadow-vsock --test streaming_exec_tests -- --nocapture

echo "[6/8] Running shadow-vmm virtio-fs & DAX I/O tests..."
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

echo "[7/8] Running shadow-vmm cold boot provisioning benchmarks..."
cargo test -p shadow-vmm --test cold_boot_benchmarks -- --nocapture

echo "[8/9] Running shadow-cow ephemeral isolation & zero pollution tests..."
cargo test -p shadow-cow --test cow_isolation_tests -- --nocapture

echo "[9/9] Running shadow-snapshot state rollback & checkpoint tests..."
cargo test -p shadow-snapshot --test snapshot_rollback_tests -- --nocapture

echo "\n--- Cross-Platform Verification & NFR Benchmarks ---"
python3 scripts/verify_phase1_protocol.py
python3 scripts/verify_phase1_vmm_mock.py
python3 scripts/benchmark_virtiofs_io.py
python3 scripts/benchmark_vsock_streaming.py
python3 scripts/benchmark_phase1_gate.py
python3 scripts/verify_milestone2_1_cow.py
python3 scripts/benchmark_milestone2_2_rollback.py

echo "=========================================================="
echo " [OK] All ShadowOS Test Suites & Gate Validations Passed!"
echo "=========================================================="
