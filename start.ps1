# ==============================================================================
# ShadowOS — Unified Full-Stack Application Launcher (PowerShell)
# Starts the Axum backend daemon (port 3001) & React dashboard (port 5173) at once.
# ==============================================================================

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ConfirmPreference = 'None'
$ProgressPreference = 'SilentlyContinue'
$Host.UI.RawUI.WindowTitle = "ShadowOS Full-Stack Platform"

Write-Host '=======================================================================' -ForegroundColor Cyan
Write-Host '   ___ _            _             ___  ____   ' -ForegroundColor Cyan
Write-Host '  / __| |_  __ _ __| |_____ __ __/ _ \/ ___|  ' -ForegroundColor Cyan
Write-Host '  \__ \ '' \/ _` / _` / _ \ V  V / (_) \___ \  ' -ForegroundColor Cyan
Write-Host '  |___/_||_\__,_\__,_\___/\_/\_/ \___/|____/  ' -ForegroundColor Cyan
Write-Host '  Hardware-Isolated MicroVM Execution & Autonomous Agent Sandbox' -ForegroundColor DarkCyan
Write-Host '=======================================================================' -ForegroundColor Cyan

$WorkspaceRoot = $PSScriptRoot
$DashboardDir = Join-Path $WorkspaceRoot "dashboard"

# 1. Resolve Cargo / Rust binary
$CargoCmd = "cargo"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $UserCargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
    if (Test-Path $UserCargo) {
        $CargoCmd = $UserCargo
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
    } else {
        Write-Host "[!] Error: Rust / Cargo is not found in PATH or $env:USERPROFILE\.cargo\bin" -ForegroundColor Red
        Write-Host "    Install Rust from https://rustup.rs before launching." -ForegroundColor Yellow
        exit 1
    }
}

# 2. Resolve Node / npm
if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "[!] Error: Node.js / npm is not installed or not in PATH." -ForegroundColor Red
    Write-Host "    Install Node.js (v18+) from https://nodejs.org before launching." -ForegroundColor Yellow
    exit 1
}

# 3. Ensure dashboard dependencies are installed
if (-not (Test-Path (Join-Path $DashboardDir "node_modules"))) {
    Write-Host "`n[*] Installing dashboard dependencies (npm install)..." -ForegroundColor Yellow
    Push-Location $DashboardDir
    try {
        & npm install
    } finally {
        Pop-Location
    }
}

# 4. Check for existing instances on port 3001 or 5173
$ExistingBackend = Get-NetTCPConnection -LocalPort 3001 -ErrorAction SilentlyContinue
if ($ExistingBackend) {
    Write-Host "[i] Note: A service is already listening on port 3001. Using existing backend." -ForegroundColor Gray
}

# 5. Start Backend Server
$BackendProcess = $null
if (-not $ExistingBackend) {
    Write-Host "`n[1/3] Starting ShadowOS Backend Server (crates/shadow-server on port 3001)..." -ForegroundColor Yellow
    $ServerExe = Join-Path $WorkspaceRoot "target\debug\shadow-server.exe"

    if (Test-Path $ServerExe) {
        Write-Host "      [+] Fast-boot: Using compiled binary target\debug\shadow-server.exe" -ForegroundColor Green
        $BackendProcess = Start-Process -FilePath $ServerExe `
            -ArgumentList @("--port", "3001", "--host", "127.0.0.1", "--workspace", $WorkspaceRoot) `
            -WorkingDirectory $WorkspaceRoot `
            -PassThru -NoNewWindow
    } else {
        Write-Host "      [+] Booting via cargo run -p shadow-server..." -ForegroundColor Green
        $BackendProcess = Start-Process -FilePath $CargoCmd `
            -ArgumentList @("run", "-p", "shadow-server", "--", "--port", "3001", "--host", "127.0.0.1", "--workspace", $WorkspaceRoot) `
            -WorkingDirectory $WorkspaceRoot `
            -PassThru -NoNewWindow
    }

    # Wait for backend health check
    Write-Host "      [*] Waiting for backend health check at http://127.0.0.1:3001/api/health..." -ForegroundColor Gray
    $BackendReady = $false
    for ($i = 0; $i -lt 30; $i++) {
        try {
            $resp = Invoke-RestMethod -Uri "http://127.0.0.1:3001/api/health" -TimeoutSec 1 -UseBasicParsing -ErrorAction SilentlyContinue
            if ($resp.status -eq "healthy") {
                $BackendReady = $true
                break
            }
        } catch {
            Start-Sleep -Milliseconds 400
        }
    }

    if ($BackendReady) {
        Write-Host "      [OK] Backend server is healthy and ready on http://127.0.0.1:3001" -ForegroundColor Green
    } else {
        Write-Host "      [!] Warning: Backend health check timed out. Proceeding..." -ForegroundColor Yellow
    }
}

# 6. Start Dashboard Frontend
Write-Host "`n[2/3] Starting ShadowOS React Dashboard (Vite on port 5173)..." -ForegroundColor Yellow
$FrontendProcess = Start-Process -FilePath "npm.cmd" `
    -ArgumentList @("run", "dev") `
    -WorkingDirectory $DashboardDir `
    -PassThru -NoNewWindow

# Wait for frontend readiness
Write-Host "      [*] Waiting for dashboard dev server at http://localhost:5173..." -ForegroundColor Gray
$FrontendReady = $false
for ($i = 0; $i -lt 30; $i++) {
    try {
        $tcp = New-Object System.Net.Sockets.TcpClient
        $iar = $tcp.BeginConnect("127.0.0.1", 5173, $null, $null)
        $wait = $iar.AsyncWaitHandle.WaitOne(400, $false)
        if ($wait) {
            $tcp.EndConnect($iar)
            $tcp.Close()
            $FrontendReady = $true
            break
        }
        $tcp.Close()
    } catch {
        Start-Sleep -Milliseconds 400
    }
}

# 7. Open Browser
Write-Host "`n[3/3] Opening dashboard in browser..." -ForegroundColor Yellow
Start-Process "http://localhost:5173"

Write-Host "`n=======================================================================" -ForegroundColor Green
Write-Host " [OK] ShadowOS Full-Stack Platform is RUNNING!" -ForegroundColor Green
Write-Host "=======================================================================" -ForegroundColor Green
Write-Host "  * Dashboard UI:       http://localhost:5173" -ForegroundColor Cyan
Write-Host "  * Backend REST API:   http://127.0.0.1:3001/api" -ForegroundColor Cyan
Write-Host "  * Live Vsock Stream:  ws://127.0.0.1:3001/api/ws/logs" -ForegroundColor Cyan
Write-Host "  * VM Terminal Stream: ws://127.0.0.1:3001/api/ws/terminal/:id" -ForegroundColor Cyan
Write-Host "`nPress Ctrl+C to stop both backend and dashboard processes.`n" -ForegroundColor Gray

# Cleanup Handler for Clean Shutdown on Ctrl+C
try {
    while ($true) {
        Start-Sleep -Seconds 1
        if ($BackendProcess -and $BackendProcess.HasExited) {
            Write-Host "[!] Backend server process exited unexpectedly." -ForegroundColor Red
            break
        }
        if ($FrontendProcess -and $FrontendProcess.HasExited) {
            Write-Host "[!] Frontend dev server process exited unexpectedly." -ForegroundColor Red
            break
        }
    }
} finally {
    Write-Host "`n[*] Shutting down ShadowOS services..." -ForegroundColor Yellow
    if ($BackendProcess -and (-not $BackendProcess.HasExited)) {
        Stop-Process -Id $BackendProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "  [OK] Stopped Backend Server (PID $($BackendProcess.Id))" -ForegroundColor Gray
    }
    if ($FrontendProcess -and (-not $FrontendProcess.HasExited)) {
        # Kill child node processes if needed
        Stop-Process -Id $FrontendProcess.Id -Force -ErrorAction SilentlyContinue
        Write-Host "  [OK] Stopped Dashboard Dev Server (PID $($FrontendProcess.Id))" -ForegroundColor Gray
    }
    Write-Host "[OK] Shutdown complete. Goodbye!`n" -ForegroundColor Green
}
