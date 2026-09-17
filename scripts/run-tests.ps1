# ==============================================================================
# Project ShadowOS — Master Test & Benchmark Suite Runner (PowerShell)
# ==============================================================================
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Running ShadowOS Test Suites & NFR Benchmarks" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Write-Host "`n[1/8] Running shadow-core unit & protocol integration tests..." -ForegroundColor Yellow
cargo test -p shadow-core --test protocol_test -- --nocapture

Write-Host "`n[2/8] Running shadow-core NFR framing benchmarks..." -ForegroundColor Yellow
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

Write-Host "`n[3/8] Running shadow-vmm lifecycle state tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

Write-Host "`n[4/8] Running shadow-vsock channel queue tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

Write-Host "`n[5/8] Running shadow-vsock streaming bash execution tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test streaming_exec_tests -- --nocapture

Write-Host "`n[6/8] Running shadow-vmm virtio-fs & DAX I/O tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

Write-Host "`n[7/8] Running shadow-vmm cold boot provisioning benchmarks..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test cold_boot_benchmarks -- --nocapture

Write-Host "`n[8/9] Running shadow-cow ephemeral isolation & zero pollution tests..." -ForegroundColor Yellow
cargo test -p shadow-cow --test cow_isolation_tests -- --nocapture

Write-Host "`n[9/9] Running shadow-snapshot state rollback & checkpoint tests..." -ForegroundColor Yellow
cargo test -p shadow-snapshot --test snapshot_rollback_tests -- --nocapture

Write-Host "`n--- Cross-Platform Verification & NFR Benchmarks ---" -ForegroundColor Cyan
python scripts/verify_phase1_protocol.py
python scripts/verify_phase1_vmm_mock.py
python scripts/benchmark_virtiofs_io.py
python scripts/benchmark_vsock_streaming.py
python scripts/benchmark_phase1_gate.py
python scripts/verify_milestone2_1_cow.py
python scripts/benchmark_milestone2_2_rollback.py

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host " [OK] All ShadowOS Test Suites & Gate Validations Passed!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
