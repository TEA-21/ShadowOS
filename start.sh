#!/usr/bin/env bash
# ==============================================================================
# ShadowOS — Unified Full-Stack Application Launcher (POSIX / Linux / macOS)
# ==============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DASHBOARD_DIR="$SCRIPT_DIR/dashboard"

echo "======================================================================="
echo "   ___ _            _             ___  ____   "
echo "  / __| |_  __ _ __| |_____ __ __/ _ \/ ___|  "
echo "  \__ \ ' \/ _\` / _\` / _ \ V  V / (_) \___ \  "
echo "  |___/_||_\__,_\__,_\___/\_/\_/ \___/|____/  "
echo "  Hardware-Isolated MicroVM Execution & Autonomous Agent Sandbox"
echo "======================================================================="

# Check dependencies
command -v cargo >/dev/null 2>&1 || { echo "[!] Error: cargo is not installed. Install Rust via https://rustup.rs"; exit 1; }
command -v npm >/dev/null 2>&1 || { echo "[!] Error: npm is not installed. Install Node.js via https://nodejs.org"; exit 1; }

# Install dashboard dependencies if missing
if [ ! -d "$DASHBOARD_DIR/node_modules" ]; then
    echo "[*] Installing dashboard dependencies (npm install)..."
    (cd "$DASHBOARD_DIR" && npm install)
fi

BACKEND_PID=""
FRONTEND_PID=""

cleanup() {
    echo ""
    echo "[*] Shutting down ShadowOS services..."
    [ -n "$BACKEND_PID" ] && kill "$BACKEND_PID" 2>/dev/null || true
    [ -n "$FRONTEND_PID" ] && kill "$FRONTEND_PID" 2>/dev/null || true
    echo "[✓] Shutdown complete. Goodbye!"
    exit 0
}

trap cleanup EXIT INT TERM

# Start Backend Server
echo ""
echo "[1/3] Starting ShadowOS Backend Server (crates/shadow-server on port 3001)..."
SERVER_BIN="$SCRIPT_DIR/target/debug/shadow-server"
if [ -f "$SERVER_BIN" ]; then
    echo "      [+] Fast-boot: Using compiled binary target/debug/shadow-server"
    "$SERVER_BIN" --port 3001 --host 127.0.0.1 --workspace "$SCRIPT_DIR" &
    BACKEND_PID=$!
else
    echo "      [+] Booting via cargo run -p shadow-server..."
    cargo run -p shadow-server -- --port 3001 --host 127.0.0.1 --workspace "$SCRIPT_DIR" &
    BACKEND_PID=$!
fi

# Poll Backend health
echo "      [*] Waiting for backend health check at http://127.0.0.1:3001/api/health..."
for i in {1..30}; do
    if curl -s http://127.0.0.1:3001/api/health >/dev/null 2>&1; then
        echo "      [✓] Backend server is healthy on http://127.0.0.1:3001"
        break
    fi
    sleep 0.4
done

# Start Dashboard Frontend
echo ""
echo "[2/3] Starting ShadowOS React Dashboard (Vite on port 5173)..."
(cd "$DASHBOARD_DIR" && npm run dev) &
FRONTEND_PID=$!

# Poll Frontend readiness
echo "      [*] Waiting for dashboard dev server at http://localhost:5173..."
for i in {1..30}; do
    if curl -s http://localhost:5173 >/dev/null 2>&1; then
        break
    fi
    sleep 0.4
done

# Open Browser
echo ""
echo "[3/3] Opening dashboard in browser..."
if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "http://localhost:5173" >/dev/null 2>&1 || true
elif command -v open >/dev/null 2>&1; then
    open "http://localhost:5173" >/dev/null 2>&1 || true
fi

echo ""
echo "======================================================================="
echo " [✓] ShadowOS Full-Stack Platform is RUNNING!"
echo "======================================================================="
echo "  * Dashboard UI:       http://localhost:5173"
echo "  * Backend REST API:   http://127.0.0.1:3001/api"
echo "  * Live Vsock Stream:  ws://127.0.0.1:3001/api/ws/logs"
echo "  * VM Terminal Stream: ws://127.0.0.1:3001/api/ws/terminal/:id"
echo ""
echo "Press Ctrl+C to stop both backend and dashboard processes."
echo ""

wait
