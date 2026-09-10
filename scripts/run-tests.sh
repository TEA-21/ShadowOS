#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Phase 1 Test & Benchmark Suite Runner
# ==============================================================================
set -euo pipefail

echo "=========================================================="
echo " Running ShadowOS Phase 1 Test Suites & NFR Benchmarks"
echo "=========================================================="

echo "[1/4] Running shadow-core unit & protocol integration tests..."
cargo test -p shadow-core --test protocol_test -- --nocapture

echo "[2/4] Running shadow-core NFR framing benchmarks..."
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

echo "[3/4] Running shadow-vmm lifecycle state tests..."
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

echo "[4/4] Running shadow-vsock multiplexer tests..."
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

echo "=========================================================="
echo " [✓] All Phase 1 Test Suites Passed Successfully!"
echo "=========================================================="
