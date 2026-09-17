#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Master Test & Benchmark Suite Runner
# ==============================================================================
set -euo pipefail

echo "=========================================================="
echo " Running ShadowOS Test Suites & NFR Benchmarks"
echo "=========================================================="

echo "[1/12] Running shadow-core unit & protocol integration tests..."
cargo test -p shadow-core --test protocol_test -- --nocapture

echo "[2/12] Running shadow-core NFR framing benchmarks..."
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

echo "[3/12] Running shadow-vmm lifecycle state tests..."
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

echo "[4/12] Running shadow-vsock channel queue tests..."
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

echo "[5/12] Running shadow-vsock streaming bash execution tests..."
cargo test -p shadow-vsock --test streaming_exec_tests -- --nocapture

echo "[6/12] Running shadow-vmm virtio-fs & DAX I/O tests..."
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

echo "[7/12] Running shadow-vmm cold boot provisioning benchmarks..."
cargo test -p shadow-vmm --test cold_boot_benchmarks -- --nocapture

echo "[8/12] Running shadow-cow ephemeral isolation & zero pollution tests..."
cargo test -p shadow-cow --test cow_isolation_tests -- --nocapture

echo "[9/12] Running shadow-snapshot state rollback & checkpoint tests..."
cargo test -p shadow-snapshot --test snapshot_rollback_tests -- --nocapture

echo "[10/12] Running shadow-cli harness & auto-approve execution tests..."
cargo test -p shadow-cli --test cli_execution_tests -- --nocapture

echo "[11/12] Running shadow-tui diff inspector & keyboard simulation tests..."
cargo test -p shadow-tui --test tui_interaction_tests -- --nocapture

echo "[12/12] Running shadow-mcp Model Context Protocol (MCP) server tests..."
cargo test -p shadow-mcp --test mcp_protocol_tests -- --nocapture

echo "\n--- Cross-Platform Verification & NFR Benchmarks ---"
python3 scripts/verify_phase1_protocol.py
python3 scripts/verify_phase1_vmm_mock.py
python3 scripts/benchmark_virtiofs_io.py
python3 scripts/benchmark_vsock_streaming.py
python3 scripts/benchmark_phase1_gate.py
python3 scripts/verify_milestone2_1_cow.py
python3 scripts/benchmark_milestone2_2_rollback.py
python3 scripts/verify_milestone2_3_cli.py
python3 scripts/verify_milestone2_4_tui.py
python3 scripts/benchmark_phase2_gate.py
python3 scripts/verify_milestone3_1_mcp.py

echo "=========================================================="
echo " [OK] All ShadowOS Test Suites & Gate Validations Passed!"
echo "=========================================================="
