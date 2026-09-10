#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Phase 1 Test & Benchmark Suite Runner
# ==============================================================================
set -euo pipefail

echo "=========================================================="
echo " Running ShadowOS Phase 1 Test Suites & NFR Benchmarks"
echo "=========================================================="

echo "[1/5] Running shadow-core unit & protocol integration tests..."
cargo test -p shadow-core --test protocol_test -- --nocapture

echo "[2/5] Running shadow-core NFR framing benchmarks..."
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

echo "[3/5] Running shadow-vmm lifecycle state tests..."
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

echo "[4/5] Running shadow-vsock multiplexer tests..."
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

echo "[5/5] Running shadow-vmm virtio-fs & DAX I/O tests..."
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

echo "\n--- Cross-Platform Verification & I/O Benchmarks ---"
python3 scripts/verify_phase1_protocol.py
python3 scripts/verify_phase1_vmm_mock.py
python3 scripts/benchmark_virtiofs_io.py

echo "=========================================================="
echo " [OK] All Phase 1 Test Suites & Benchmarks Passed!"
echo "=========================================================="
