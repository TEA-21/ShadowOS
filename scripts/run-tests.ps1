# ==============================================================================
# Project ShadowOS — Phase 1 Test & Benchmark Suite Runner (PowerShell)
# ==============================================================================
$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " Running ShadowOS Phase 1 Test Suites & NFR Benchmarks" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

Write-Host "`n[1/4] Running shadow-core unit & protocol integration tests..." -ForegroundColor Yellow
cargo test -p shadow-core --test protocol_test -- --nocapture

Write-Host "`n[2/4] Running shadow-core NFR framing benchmarks..." -ForegroundColor Yellow
cargo test -p shadow-core --test benchmark_nfr -- --nocapture

Write-Host "`n[3/4] Running shadow-vmm lifecycle state tests..." -ForegroundColor Yellow
cargo test -p shadow-vmm --test vmm_lifecycle_tests -- --nocapture

Write-Host "`n[4/4] Running shadow-vsock multiplexer tests..." -ForegroundColor Yellow
cargo test -p shadow-vsock --test vsock_tests -- --nocapture

Write-Host "`n==========================================================" -ForegroundColor Green
Write-Host " [✓] All Phase 1 Test Suites Passed Successfully!" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
