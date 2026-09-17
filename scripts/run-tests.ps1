# ==============================================================================
# Project ShadowOS — Master Test & Benchmark Suite Runner (PowerShell)
# ==============================================================================
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Running ShadowOS Test Suites & NFR Benchmarks" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Write-Host "`n[1/16] Running shadow-core unit & protocol integration tests..." -ForegroundColor Yellow
cargo test -p shadow-core --test protocol_test -- --nocapture

Write-Host "`n[2/16] Running shadow-core NFR framing benchmarks..." -ForegroundColor Yellow
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

Write-Host "`n[3/16] Running shadow-vmm lifecycle state tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

Write-Host "`n[4/16] Running shadow-vsock channel queue tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

Write-Host "`n[5/16] Running shadow-vsock streaming bash execution tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test streaming_exec_tests -- --nocapture

Write-Host "`n[6/16] Running shadow-vmm virtio-fs & DAX I/O tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

Write-Host "`n[7/16] Running shadow-vmm cold boot provisioning benchmarks..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test cold_boot_benchmarks -- --nocapture

Write-Host "`n[8/16] Running shadow-cow ephemeral isolation & zero pollution tests..." -ForegroundColor Yellow
cargo test -p shadow-cow --test cow_isolation_tests -- --nocapture

Write-Host "`n[9/16] Running shadow-snapshot state rollback & checkpoint tests..." -ForegroundColor Yellow
cargo test -p shadow-snapshot --test snapshot_rollback_tests -- --nocapture

Write-Host "`n[10/16] Running shadow-cli harness & auto-approve execution tests..." -ForegroundColor Yellow
cargo test -p shadow-cli --test cli_execution_tests -- --nocapture

Write-Host "`n[11/16] Running shadow-tui diff inspector & keyboard simulation tests..." -ForegroundColor Yellow
cargo test -p shadow-tui --test tui_interaction_tests -- --nocapture

Write-Host "`n[12/16] Running shadow-mcp Model Context Protocol (MCP) server tests..." -ForegroundColor Yellow
cargo test -p shadow-mcp --test mcp_protocol_tests -- --nocapture

Write-Host "`n[13/16] Running shadow-vmm headless virtual display & raw CDP tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test display_cdp_tests -- --nocapture

Write-Host "`n[14/16] Running shadow-mcp visual browser tools tests..." -ForegroundColor Yellow
cargo test -p shadow-mcp --test mcp_browser_tests -- --nocapture

Write-Host "`n[15/16] Running shadow-vmm swarm orchestrator & virtio-balloon tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test swarm_orchestration_tests -- --nocapture

Write-Host "`n[16/16] Running shadow-mcp swarm primitives tests..." -ForegroundColor Yellow
cargo test -p shadow-mcp --test mcp_swarm_tests -- --nocapture

Write-Host "`n--- Cross-Platform Verification & NFR Benchmarks ---" -ForegroundColor Cyan
python scripts/verify_phase1_protocol.py
python scripts/verify_phase1_vmm_mock.py
python scripts/benchmark_virtiofs_io.py
python scripts/benchmark_vsock_streaming.py
python scripts/benchmark_phase1_gate.py
python scripts/verify_milestone2_1_cow.py
python scripts/benchmark_milestone2_2_rollback.py
python scripts/verify_milestone2_3_cli.py
python scripts/verify_milestone2_4_tui.py
python scripts/benchmark_phase2_gate.py
python scripts/verify_milestone3_1_mcp.py
python scripts/verify_milestone3_2_display.py
python scripts/benchmark_milestone3_3_swarm.py

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host " [OK] All ShadowOS Test Suites & Gate Validations Passed!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
