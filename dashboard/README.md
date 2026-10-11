# ShadowOS Dashboard

> **Modern, data-dense, light-themed developer dashboard for the ShadowOS microVM and sandboxing platform.**  
> Manages hardware-isolated microVMs (`libkrun` / `Firecracker`), Copy-on-Write (COW) 4-layer OverlayFS filesystems, sub-100ms memory rollbacks, and Model Context Protocol (MCP) swarm orchestration.

---

## 📖 Complete Documentation

For an exhaustive guide covering the purpose and functionality of every tab, button, modal, and architecture decision, please see:

👉 **[FRONTEND_GUIDE.md](./FRONTEND_GUIDE.md)**

---

## ⚡ Tech Stack & Design System

* **Framework:** [React 19](https://react.dev/) + [TypeScript](https://www.typescriptlang.org/)
* **Build Tool:** [Vite 8](https://vitejs.dev/)
* **Styling:** [Tailwind CSS](https://tailwindcss.com/)
* **Icons:** [Lucide React](https://lucide.dev/)
* **Theme:** Strict Light Theme (`bg-slate-50` background, pure white `bg-white` cards)
* **Primary Accent:** Vibrant Purple (`#6B46C1` / `bg-purple-600`)
* **Status Colors:** Muted Green (`#2E8B57`) for Active, Soft Orange (`#D97757`) for Suspended/Warning
* **Typography:** **Inter** for UI text, **JetBrains Mono** for VM IDs, diffs, metrics, and terminal logs

---

## 🚀 Quick Start

### Prerequisites
* Node.js `>= 18.0.0`
* npm `>= 9.0.0`

### Installation
```bash
# Navigate to the dashboard directory
cd dashboard

# Install dependencies
npm install

# Start development server
npm run dev
```

The application will be running at `http://localhost:5173/` (or `http://localhost:5174/`).

### Production Build
```bash
npm run build
```

---

## 🧭 Platform Views

1. **Sandbox Manager (`/sandboxes`)**:
   - High-density microVM data table with status indicators, memory usage bars, and pinned hardware core metrics.
   - Quick actions: `Pause/Resume`, `Terminal` (vsock pseudo-terminal modal), `Kill`, and `+ Spawn Sandbox` provisioning form.

2. **Snapshot & Rollback View (`/snapshots`)**:
   - 30% / 70% split layout.
   - Left: Historical checkpoints timeline with memory deltas and rollback timings.
   - Right: Redesigned Git Diff component showing Copy-on-Write additions (emerald), modifications (orange), and deletions (red), with collapsible AI Diff Explanations and a prominent `Rollback to State` button.

3. **MCP / Swarm Console (`/mcp-console`)**:
   - 50% / 50% split view layout.
   - Left: Live monospace stream of vsock packets and JSON-RPC 2.0 frames with expandable payload viewers.
   - Right: Agent Execution Summary with test pass metrics (24/24 PASS), NFR audits, and working `Inspect Diff in Staging` and `Promote to Host Tree` actions with atomic confirmation modal.
