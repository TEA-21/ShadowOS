# ShadowOS Dashboard — Comprehensive Frontend Architecture & UI Specification

> **A minimal, high-focus, light-themed developer platform dashboard for the ShadowOS microVM and sandboxing platform.**  
> Built with **React 19**, **TypeScript**, **Tailwind CSS**, and **Lucide React**.

---

## 1. Executive Overview & Design System

The ShadowOS Dashboard provides an interface for managing hardware-isolated microVM execution sandboxes (Firecracker / libkrun), Copy-on-Write (COW) 4-layer OverlayFS filesystems, sub-100ms memory rollbacks, and Model Context Protocol (MCP) multi-agent swarm orchestration.

### Design Tokens & Palette

| Token | Class / Hex | Visual Role |
| :--- | :--- | :--- |
| **Base App Background** | `bg-slate-50` (`#F8FAFC`) | Very light, non-distracting off-white base maximizing data clarity. |
| **Cards & Panels** | `bg-white` (`#FFFFFF`) | Pure white surfaces with thin `border-slate-200` borders and subtle drop shadows (`shadow-subtle`). |
| **Primary Accent** | `bg-purple-600` / `#6B46C1` | Primary action buttons (`Spawn Sandbox`, `Rollback to State`, `Promote to Host Tree`), active tab pills, tags, and progress bars. |
| **Status: Active / Success** | `text-[#2E8B57]` / `bg-emerald-50` | Active microVMs, passing test audits, verified SHA256 checksums, and added diff lines. |
| **Status: Warning / Note** | `text-[#D97757]` / `bg-amber-50` | Suspended sandboxes, in-progress agent tasks, and modified file diff hunks. |
| **Status: Error / Deleted** | `text-red-700` / `bg-red-50` | Stopped instances, deleted files, and negative diff lines. |
| **Typography (UI)** | **Inter** (`font-sans`) | Clean, high-legibility sans-serif for labels, headings, stats, and buttons. |
| **Typography (Code & Data)** | **JetBrains Mono** (`font-mono`) | Monospace font strictly applied to VM IDs, vsock channels, code diffs, timings, and terminal logs. |

---

## 2. Global Layout & Shell Architecture

The dashboard is structured as a responsive layout with a fixed sidebar, sticky top header, and scrollable main content viewport.

### A. Top Header (`Header.tsx`)
The top bar provides global navigation and action controls:
1. **View Title:** Displays the name of the currently active tab (`Sandbox Manager`, `Snapshot & Rollback`, `MCP / Swarm Console`).
2. **Workspace Selector Dropdown:**
   - **Purpose:** Switches between active compute clusters (`prod-swarm-cluster (Active)`, `staging-eval-pool`, `local-libkrun-ephemeral`).
   - **Options:** Allows isolation between production worker pools and local testing sandboxes.
3. **Global Search Input:**
   - **Purpose:** Quick-filter search across microVM IDs, modified file paths, and terminal logs.
   - **Placeholder:** *"Search VMs, files, hunks..."*
4. **Swarm Memory Indicator:**
   - **Purpose:** Live quota tracking of aggregate swarm RAM usage against the platform's 2.50 GB hard cap.
   - **Visualization:** Shows exact numerical footprint (`760 MB / 2500 MB`) with an adaptive purple progress bar.
5. **Primary Action Button (`+ Spawn Sandbox`):**
   - **Purpose:** The single global button to provision a new microVM.
   - **Interaction:** Opens the `SpawnSandboxModal`.

### B. Left Sidebar Navigation (`Sidebar.tsx`)
The sidebar controls platform routing and shows connectivity status:
1. **Brand Identity:** ShadowOS logo, platform name, and `v1.0` badge.
2. **Navigation Items (Strictly 3 Views):**
   - **Sandbox Manager:** Directs to the active microVM grid. Displays live pill badge (`N Active`).
   - **Snapshot & Rollback:** Directs to the CoW diff and state rollback inspector. Displays pill badge (`N States`).
   - **MCP / Swarm Console:** Directs to the vsock log stream and AI agent summary. Displays warning pill badge (`Live`).
3. **Subsystem Status Footer:**
   - Shows active `AF_VSOCK` channel range (`CID: 3..7`) and active hypervisor drivers (`libkrun / FC`).

---

## 3. Tab 1: Sandbox Manager (Active Environments)

**File:** `src/components/sandbox/SandboxManager.tsx`  
**Purpose:** Provides a centralized control center for monitoring, configuring, interacting with, and terminating running microVM execution environments.

### 1. High-Focus KPI Strip
Four summary cards at the top of the view:
* **Running MicroVMs:** Displays active count vs total provisioned instances (e.g. `3 / 5 instances`).
* **Suspended / Idle:** Shows microVMs that have been paused with memory ballooned down to idle footprints.
* **Swarm RAM Usage:** Real-time aggregate memory consumption against the limit.
* **Cold Boot Latency:** Average provisioning speed (`22.38 ms avg`), highlighting sub-150ms performance.

### 2. Toolbar & Filtering
* **Status Filter Pills (`All`, `Active`, `Suspended`, `Stopped`):**
  - Filters the table or grid in real-time. Each pill displays the live instance count in parentheses.
* **View Switcher (`Table` / `Grid`):**
  - **Table View Button:** Compact, structured data table for high-density monitoring.
  - **Grid View Button:** Card-based layout with visual memory bars per microVM.

### 3. Data Table Columns & Row Structure
The table is designed for maximum clarity, with low-level clutter removed:
* **VM ID / Name:** Monospace ID badge (e.g., `vm-4a7b2e9c`), human-readable name (e.g., `agent-workspace-primary`), and hypervisor tag (`libkrun` or `firecracker`).
* **Status:** Clean dot badge:
  - `Active`: Green (`#2E8B57`)
  - `Suspended`: Soft orange (`#D97757`)
  - `Stopped`: Neutral slate
* **Uptime:** Monospace string indicating elapsed execution time (e.g., `1h 42m 18s`).
* **RAM Usage:** Exact RSS footprint (e.g., `140 MB / 512 MB`) accompanied by a mini purple progress bar.
* **CPU Allocation:** Number of vCPUs, utilization percentage, and pinned hardware core badge (`Core #0`).
* **Actions (Compact Muted Outline Button Group):**
  - **`Pause / Resume` Button:**
    - If `Active`: Suspends the guest microVM, frees CPU cycles, and triggers dynamic memory ballooning.
    - If `Suspended`: Resumes execution without restarting the guest kernel.
  - **`Terminal` Button:**
    - Opens the interactive `VmTerminalModal` connected to that microVM's dedicated vsock channel.
  - **`Kill` Button:**
    - Safely terminates the microVM instance, purging ephemeral runtime memory.

### 4. Modals in Sandbox Manager

#### A. Provision MicroVM Modal (`SpawnSandboxModal.tsx`)
Triggered by the `+ Spawn Sandbox` button:
* **Sandbox Name Input:** Assigns an identifier to the worker sandbox.
* **Hypervisor Selection Buttons:**
  - `libkrun Driver`: In-process TSI virtualization for sub-25ms cold boots.
  - `Firecracker VMM`: Minimal seccomp-jailed Linux microVM.
* **vCPU Cores Dropdown:** Configures hardware CPU allocation (1, 2, or 4 vCPUs).
* **RAM Allocation Dropdown:** Configures guest memory ceiling (256 MB, 512 MB, 1024 MB, 2048 MB).
* **Workload Execution Profile:** Preset configurations (`DAX Direct-Map + 4-Layer CoW`, `Headless Virtual Display Xvfb`, `High-Throughput Swarm Runner`).
* **virtio-fs DAX Toggle:** Enables direct host memory mapping bypass for guest caching.
* **Action Buttons:** `Cancel` (dismisses modal) and `Spawn MicroVM` (provisions instance).

#### B. VM Terminal Modal (`VmTerminalModal.tsx`)
Triggered by the `Terminal` action button:
* **Header:** Displays VM ID, name, and connected vsock CID with connection status.
* **Monospace Terminal Window:**
  - Displays real-time multiplexed stdout/stderr output.
  - Shows kernel boot string and guest agent handshakes.
* **Command Input Bar:**
  - Type commands (`cargo check`, `ls -la`, `free -m`, `exit`) to simulate guest shell execution.
* **Action Buttons:**
  - `Clear`: Empties the current terminal scroll buffer.
  - `Close Session`: Disconnects and closes the terminal modal.

---

## 4. Tab 2: Snapshot & Rollback View (State Management)

**Files:** `src/components/snapshots/SnapshotRollbackView.tsx` & `src/components/snapshots/FileSystemDiffViewer.tsx`  
**Purpose:** Inspects historical checkpoints of a sandbox, reviews Copy-on-Write (COW) overlay file changes, and executes sub-100ms state restorations.

### 1. Top Environment Bar
* **Target Sandbox Selector:** Dropdown selecting which sandbox's snapshots to inspect.
* **`+ Create Checkpoint` Button:** Captures a manual point-in-time snapshot of the sandbox state.
* **Rollback Success Banner:** When a rollback is executed, shows an emerald banner confirming latency (`31.05 ms`) and bit-identical host repository integrity.

### 2. Left Column (30% Width): Checkpoints History List
A vertical timeline of saved checkpoints:
* **Checkpoint Card Components:**
  - **Snapshot ID:** Monospace identifier (e.g. `snap-0842-ast-refactor`).
  - **Trigger Badge:**
    - `Milestone`: Purple badge for autonomous PRD milestones.
    - `Pre-Build`: Orange badge for snapshots generated before build steps.
    - `Swarm`: Green badge for swarm synchronization points.
    - `Manual`: Gray badge for user-triggered checkpoints.
  - **Timestamp:** Time of capture.
  - **Title & Description:** Context on what occurred prior to the checkpoint.
  - **Changes Count:** Number of files modified in the ephemeral upperdir.
  - **Rollback Latency:** Target restoration speed benchmark (e.g. `31.05 ms`).

### 3. Right Column (70% Width): Redesigned Git Diff Viewer
Inspired by code review tools (GitHub / GitLab / Linear):

#### A. Diff Toolbar
* **File Selector Dropdown:** Switches between modified files in the snapshot.
* **File Status Badge:** Indicates whether the file was `Added` (green), `Modified` (orange), or `Deleted` (red).
* **Change Overview Pills:** Displays additions (`+18 lines`) and deletions (`-4 lines`).
* **`Previous File (<)` / `Next File (>)` Buttons:** Steps through the list of modified files.
* **`AI Explanation` Toggle Button:** Expands or collapses the contextual AI explanation callout.
* **`Copy Diff` Button:** Copies the unified diff of the selected file to the clipboard.
* **Prominent `Rollback to State` Button (Vibrant Purple):**
  - Triggers an instant state rollback.
  - Wipes the ephemeral `upperdir` tmpfs layer and restores memory state in `< 35 ms`.

#### B. Contextual AI Diff Explanation Panel
An expandable callout box in light purple (`bg-purple-50/80 border-purple-200`):
* Summarizes the architectural rationale behind the changes in human-readable bullet points.
* Explains core pinning, memory balloon attachment, or CLI auto-approve injections.

#### C. Code Diff Display
* **Background:** Neutral light gray (`#F8FAFC`).
* **Line Number Gutters:** Separate columns for old and new line numbers with clear right borders.
* **Hunk Header:** Slate banner displaying hunk line offsets (e.g., `@@ -42,10 +42,24 @@`).
* **Deletions (`-`):** Light red background (`bg-red-50/80`), dark red text (`text-red-900`), and red `-` prefix.
* **Additions (`+`):** Light green background (`bg-emerald-50/80`), dark green text (`text-emerald-900`), and green `+` prefix.
* **Context Lines:** Clean neutral background for unmodified context.

---

## 5. Tab 3: MCP / Swarm Console

**File:** `src/components/mcp/McpSwarmConsole.tsx`  
**Purpose:** Provides a two-pane operations console simulating Model Context Protocol (MCP) interactions between the AI agent swarm and the microVM runtime.

### 1. Left Pane (50% Width): Live Vsock Stream & MCP Events
A monospace terminal stream showing events over virtual sockets and stdio:
* **Toolbar Controls:**
  - **`Pause / Resume` Button:** Pauses incoming simulated vsock packet events.
  - **`Auto-Scroll` Button:** Toggles auto-scrolling to the latest log line.
  - **`Clear Logs` (Trash) Button:** Clears the event buffer.
* **Filter Bar:**
  - **Channel Dropdown:** Filters by subsystem (`All Channels`, `mcp-jsonrpc`, `vsock`, `virtio-balloon`, `overlayfs`, `cdp`).
  - **Level Dropdown:** Filters by severity (`All Levels`, `INFO`, `DEBUG`, `TRACE`, `WARN`).
* **Log Entry Display:**
  - Monospace timestamp, colored badge (`[INFO]`, `[DEBUG]`, `[WARN]`, `[TRACE]`), channel pill, VM ID, and log message.
  - **Clean Word Wrapping:** Uses `break-words` to prevent ugly mid-word wrapping.
  - **`JSON` Button:** Toggles an expandable view of the raw JSON-RPC 2.0 payload.

### 2. Right Pane (50% Width): Agent Execution Summary
A structured dashboard panel presenting autonomous agent progress:
* **Note Selector Dropdown:** Selects between agent reports (`Milestone 3.3 Swarm Verification`, `Headless Virtual Display Report`).
* **Top Metric Cards:**
  - **Tests Passed:** Green checkmark with test completion ratio (e.g., `24 / 24`).
  - **Compilation Time:** Clock icon with duration in milliseconds (e.g., `3,410 ms`).
  - **CoW Delta:** Layer icon showing uncommitted diff size (e.g., `12.4 MB`).
* **Task Directive & Mission Card:**
  - Describes the agent directive, coordinator identifier, and hypervisor profile.
* **Verification Audit & Performance Gates Card:**
  - Displays Non-Functional Requirement (NFR) benchmark results with green `PASS` badges.
* **Key Observations & Security Boundaries Card:**
  - Bulleted list covering zero-pollution guarantees and core affinities.
* **Recommended Actions Card & Action Buttons:**
  - **`Inspect Diff in Staging` Button (Outline Purple):**
    - Navigates directly to the **Snapshot & Rollback View** and opens the diff for inspection.
  - **`Promote to Host Tree` Button (Primary Purple):**
    - Opens the **Atomic Promotion Modal**.

### 3. Atomic Promotion Modal & Workflow
* **Modal Details:**
  - Shows target host repository (`c:\Users\HP\OneDrive\Documents\ShadowOs\ShadowOS`).
  - Lists the 6 files being committed to the host workspace with line deltas.
  - Confirms SHA256 integrity verification.
* **Action Buttons:**
  - `Cancel`: Closes modal without promoting.
  - `Confirm Atomic Promotion`: Executes atomic promotion, marks the agent task `Completed`, emits a `promote_change` JSON-RPC frame into the live log stream, and displays a success banner.

---

## 6. Directory Structure & File Manifest

```
dashboard/
├── .gitignore                   # Ignores node_modules, dist, logs
├── package.json                 # React 19, Tailwind CSS, Lucide React dependencies
├── postcss.config.js            # PostCSS configuration for Tailwind
├── tailwind.config.js           # Theme configuration (purple #6B46C1, status active #2E8B57, warning #D97757)
├── tsconfig.json                # Project TypeScript configuration
├── tsconfig.app.json            # Vite React TypeScript compiler options
├── vite.config.ts               # Vite configuration
├── index.html                   # HTML entry point with Inter & JetBrains Mono Google Fonts
├── FRONTEND_GUIDE.md            # This comprehensive frontend architecture guide
└── src/
    ├── main.tsx                 # React DOM mount point
    ├── index.css                # Tailwind directives & custom scrollbars
    ├── App.tsx                  # Root state management & routing
    ├── types/
    │   └── index.ts             # TypeScript interfaces (MicroVM, Snapshot, VsockLogEntry, AgentNote)
    ├── data/
    │   └── mockData.ts          # ShadowOS domain mock datasets
    └── components/
        ├── common/
        │   ├── Badge.tsx        # Status indicator badges
        │   ├── Button.tsx       # Standardized button variants
        │   ├── Card.tsx         # White panel container
        │   └── Modal.tsx        # Accessible dialog wrapper
        ├── layout/
        │   ├── Header.tsx       # Top bar with workspace selector, search & spawn button
        │   ├── Sidebar.tsx      # Left navigation shell
        │   └── Layout.tsx       # Responsive layout wrapper
        ├── sandbox/
        │   ├── SandboxManager.tsx   # MicroVM table & grid manager
        │   ├── SpawnSandboxModal.tsx# MicroVM provisioning form
        │   └── VmTerminalModal.tsx  # Interactive vsock pseudo-terminal
        ├── snapshots/
        │   ├── SnapshotRollbackView.tsx # Checkpoints timeline (30/70 layout)
        │   └── FileSystemDiffViewer.tsx # Redesigned Git Diff component
        ├── mcp/
        │   └── McpSwarmConsole.tsx  # Two-pane vsock stream & Agent Summary
        └── analytics/
            └── OverviewMetrics.tsx  # PRD benchmark compliance audit table
```

---

## 7. Build, Verification & GitHub Push Readiness

### A. Local Verification
The frontend compiles with zero errors:

```bash
cd dashboard
npm run build
```

Expected output:
```text
✓ 1921 modules transformed.
dist/index.html                   1.30 kB
dist/assets/index-BOJERViv.css   27.68 kB
dist/assets/index-CbIJ3B02.js   317.27 kB
✓ built in 12.13s
```

### B. Git Status & Ignored Files
Both root `.gitignore` and `dashboard/.gitignore` are configured to prevent committing generated files:
* `dashboard/node_modules/` is **ignored**.
* `dashboard/dist/` is **ignored**.
* Only source files, configs, and assets are staged for Git.

### C. Commands to Push to GitHub

```bash
# 1. Check status from repository root
git status

# 2. Stage all changes
git add .

# 3. Commit with a descriptive message
git commit -m "feat(dashboard): implement ShadowOS light-themed developer dashboard"

# 4. Push to remote repository
git push origin main
```
