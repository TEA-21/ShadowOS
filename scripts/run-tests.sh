#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Phase 1 Test & Benchmark Suite Runner
# ==============================================================================
set -euo pipefail

echo "=========================================================="
echo " Running ShadowOS Phase 1 Test Suites & NFR Benchmarks"
echo "=========================================================="

echo "[1/7] Running shadow-core unit & protocol integration tests..."
cargo test -p shadow-core --test protocol_test -- --nocapture

echo "[2/7] Running shadow-core NFR framing benchmarks..."
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

echo "[3/7] Running shadow-vmm lifecycle state tests..."
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

echo "[4/7] Running shadow-vsock channel queue tests..."
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

echo "[5/7] Running shadow-vsock streaming bash execution tests..."
cargo test -p shadow-vsock --test streaming_exec_tests -- --nocapture

echo "[6/7] Running shadow-vmm virtio-fs & DAX I/O tests..."
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

echo "[7/7] Running shadow-vmm cold boot provisioning benchmarks..."
cargo test -p shadow-vmm --test cold_boot_benchmarks -- --nocapture

echo "\n--- Cross-Platform Verification & Phase 1 Validation Gate ---"
python3 scripts/verify_phase1_protocol.py
python3 scripts/verify_phase1_vmm_mock.py
python3 scripts/benchmark_virtiofs_io.py
python3 scripts/benchmark_vsock_streaming.py
python3 scripts/benchmark_phase1_gate.py

echo "=========================================================="
echo " [OK] All Phase 1 Test Suites & Gate Validations Passed!"
echo "=========================================================="
