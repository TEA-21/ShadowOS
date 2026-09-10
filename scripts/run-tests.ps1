# ==============================================================================
# Project ShadowOS — Phase 1 Test & Benchmark Suite Runner (PowerShell)
# ==============================================================================
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Running ShadowOS Phase 1 Test Suites & NFR Benchmarks" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Write-Host "`n[1/6] Running shadow-core unit & protocol integration tests..." -ForegroundColor Yellow
cargo test -p shadow-core --test protocol_test -- --nocapture

Write-Host "`n[2/6] Running shadow-core NFR framing benchmarks..." -ForegroundColor Yellow
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

Write-Host "`n[3/6] Running shadow-vmm lifecycle state tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

Write-Host "`n[4/6] Running shadow-vsock channel queue tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

Write-Host "`n[5/6] Running shadow-vsock streaming bash execution tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test streaming_exec_tests -- --nocapture

Write-Host "`n[6/6] Running shadow-vmm virtio-fs & DAX I/O tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test virtiofs_tests -- --nocapture

Write-Host "`n--- Cross-Platform Verification & NFR Benchmarks ---" -ForegroundColor Cyan
python scripts/verify_phase1_protocol.py
python scripts/verify_phase1_vmm_mock.py
python scripts/benchmark_virtiofs_io.py
python scripts/benchmark_vsock_streaming.py

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host " [OK] All Phase 1 Test Suites & Benchmarks Passed!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
