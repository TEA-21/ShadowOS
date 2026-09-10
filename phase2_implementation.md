# Phase 2: M2 Harness Tooling — Technical Implementation Guide
*Project ShadowOS — Background MicroVM Execution Harness for Autonomous Agent Runtimes*

---

## 1. Architectural Blueprint & System Setup

Phase 2 (M2 Harness Tooling) builds the execution orchestration layer on top of the Phase 1 Core Engine. It delivers:
1. **The Host CLI Harness (`shadow-cli`):** Intercepts agent commands (e.g. `claude`, `swe-agent`), mounts workspaces, injects synthetic credentials, and passes auto-approval flags.
2. **Ephemeral Copy-on-Write (CoW) Storage Engine:** Isolates all guest file writes to an ephemeral overlay layer, keeping the host repository strictly read-only.
3. **Sub-100ms State Rollback Engine:** Checkpoints guest RAM and storage state before risky commands to instantly rewind from agent hallucinations or destructive bash operations (`rm -rf /`, broken dependencies, corrupted lockfiles).
4. **Host Promotion & Unified Git Diff UI:** Provides an interactive diff inspector and single-click patch promotion mechanism to stage verified guest changes back to the host machine.

```
+-----------------------------------------------------------------------------------------+
|                                    HOST WORKSTATION                                     |
|                                                                                         |
|  $ shadow-cli run claude --prompt "Refactor database migrations and run tests"          |
|                                                                                         |
|  +-----------------------------------------------------------------------------------+  |
|  |                            shadow-cli Engine (Rust)                               |  |
|  |                                                                                   |  |
|  |  +-------------------------------------+   +-----------------------------------+  |  |
|  |  |     Credential Sanitizer & Mock     |   |      Diff Inspector & TUI         |  |  |
|  |  |     (Strips ~/.aws, ~/.ssh keys)    |   |     (ratatui Unified Git Diff)    |  |  |
|  |  +-------------------------------------+   +-----------------------------------+  |  |
|  |                   |                                          ▲                    |  |
|  |                   ▼                                          │                    |  |
|  |  +-------------------------------------+   +-----------------------------------+  |  |
|  |  |      Snapshot & Rollback Mgr        |   |      Host Promotion Bridge        |  |  |
|  |  |   (RAM checkpoint + Overlay swap)   |   |   (Stages changes back to host)   |  |  |
|  |  +-------------------------------------+   +-----------------------------------+  |  |
|  |                   │                                          ▲                    |  |
|  |                   ▼                                          │                    |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  |                 virtiofsd (Read-Only Mount on Host Workspace)               |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  +-------------------|------------------------------------------|--------------------+  |
+----------------------|------------------------------------------|-----------------------+
                       │ AF_VSOCK Control Channel                 │
+----------------------|------------------------------------------|-----------------------+
|                      ▼                                          ▼                       |
|  +-----------------------------------------------------------------------------------+  |
|  |                               SHADOW GUEST OS (MicroVM)                           |  |
|  |                                                                                   |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  | OverlayFS Mount:                                                            |  |  |
|  |  |   lowerdir = /mnt/virtiofs-ro (Host Repo, Strictly Read-Only)               |  |  |
|  |  |   upperdir = /tmp/cow_overlay/upper (RAM-disk tmpfs, Ephemeral Writes)      |  |  |
|  |  |   workdir  = /tmp/cow_overlay/work  (tmpfs scratchpad)                      |  |  |
|  |  |   merged   = /workspace             (Agent Active Working Directory)        |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  | Agent Execution Daemon:                                                      |  |  |
|  |  |   Executes: claude -p "..." --dangerously-skip-permissions                  |  |  |
|  |  |   Environment: Injected MOCK_TOKEN, SANDBOX_ENV=1, isolated loopback        |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  +-----------------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------------+
```

---

### 1.1 Copy-on-Write (CoW) Storage Architecture

To prevent host filesystem pollution and corrupted state:
1. **Read-Only Host Export:**
   - The host repository is exported via `virtiofsd` with the read-only flag `--read-only` or mounted read-only inside the guest at `/mnt/host_repo_ro`.
2. **Guest OverlayFS Stack:**
   - An ephemeral `tmpfs` is mounted at `/tmp/cow_overlay` inside the guest MicroVM.
   - Linux OverlayFS combines the layers:
     ```bash
     mkdir -p /tmp/cow_overlay/upper /tmp/cow_overlay/work /workspace
     mount -t overlay overlay \
       -o lowerdir=/mnt/host_repo_ro,upperdir=/tmp/cow_overlay/upper,workdir=/tmp/cow_overlay/work \
       /workspace
     ```
3. **Write Isolation:**
   - Any file created, modified, or deleted by the agent (`npm install`, editing `.py`/`.rs` files, compiling binaries) is written **exclusively** to `/tmp/cow_overlay/upper`.
   - The host repository remains completely bit-identical throughout agent execution.

---

### 1.2 Sub-100ms State Rollback Mechanism

When an agent hallucinates, enters an infinite loop, or runs destructive scripts (e.g. `rm -rf /` or breaking `package.json`), ShadowOS can roll back the entire execution environment in under 100ms:

```
                          ROLLBACK SEQUENCE (<100ms)
+-----------------------------------------------------------------------------+
| 1. Pause MicroVM VCPUs:                                                     |
|    Firecracker API: PUT /vm/stop (Latency: ~4ms)                            |
+-----------------------------------------------------------------------------+
                                       │
                                       ▼
+-----------------------------------------------------------------------------+
| 2. Revert Storage Overlay:                                                  |
|    Option A: In-guest reset: umount /workspace -> rm -rf /tmp/cow_overlay/upper |
|              -> mkdir upper work -> remount overlay (<8ms)                  |
|    Option B: Pre-forked RAM snapshot restore (<25ms)                        |
+-----------------------------------------------------------------------------+
                                       │
                                       ▼
+-----------------------------------------------------------------------------+
| 3. Restore MicroVM Memory State:                                            |
|    Firecracker Snapshot Load: PUT /snapshot/load with mem_file_path         |
|    Memory-mapped from /dev/shm (zero disk I/O, pure memcpy) (Latency: ~45ms) |
+-----------------------------------------------------------------------------+
                                       │
                                       ▼
+-----------------------------------------------------------------------------+
| 4. Resume MicroVM VCPUs:                                                    |
|    Firecracker API: PUT /vm/resume (Latency: ~3ms)                          |
|    Total Rollback Elapsed Time: ~60ms (< 100ms PRD target)                  |
+-----------------------------------------------------------------------------+
```

1. **Snapshot Creation (Pre-Task Checkpoint):**
   - Before handing off a risky prompt to the agent, the host supervisor issues a snapshot request.
   - Firecracker writes the guest memory state to `/dev/shm/shadow_snap_vm01.mem` and device state to `/dev/shm/shadow_snap_vm01.state`.
   - By keeping snapshot files in `/dev/shm` (host tmpfs RAM-disk), NVMe I/O bottlenecks are eliminated.
2. **Instant Rollback Execution:**
   - If a test fails or the user rejects execution, `shadow-cli rollback` stops the VCPUs, resets the memory map from `/dev/shm`, clears the OverlayFS upperdir, and restarts the execution loop in **< 75ms**.

---

### 1.3 Transparent Agent Invocation & Credential Sanitization

1. **CLI Interception:**
   `shadow-cli run claude --prompt "<task>"` intercepts calls, performs the following pipeline:
   - Identifies local Git root directory.
   - Provisions or claims a warmed MicroVM from the pool (<150ms).
   - Mounts Git root as `virtio-fs` read-only lowerdir.
   - Creates a checkpoint snapshot (`snap_checkpoint_0`).
2. **Auto-Approve Flag Injection:**
   - Agents like Claude Code normally prompt the user: *"Do you want to run `npm test`? (y/n)"*.
   - In ShadowOS, the agent runs inside a hardware sandbox. The wrapper passes unattended flags:
     `claude -p "<prompt>" --dangerously-skip-permissions`
   - For other agents (SWE-agent, Aider, Devin-like runtimes), non-interactive / headless flags are dynamically mapped.
3. **Synthetic Credential Injection:**
   - Host ambient environment variables (`AWS_SECRET_ACCESS_KEY`, `GITHUB_TOKEN`, `OPENAI_API_KEY`) and dotfiles (`~/.ssh`, `~/.aws`, `~/.config`) are strictly stripped.
   - Only synthetic or scoped session tokens configured in `.shadow/config.json` are passed into the guest process:
     ```json
     {
       "env": {
         "NODE_ENV": "test",
         "MOCK_API_KEY": "sk-shadow-mock-998822",
         "SANDBOX": "true"
       }
     }
     ```

---

### 1.4 Host Promotion & Unified Git Diff UI

Once the agent finishes execution:
1. **Patch Extraction:**
   - The host supervisor queries the guest for altered files or reads `/tmp/cow_overlay/upper` directly via a guest helper command.
   - A unified patch is generated using `libgit2` / `similar` comparing the guest workspace state against host `HEAD`.
2. **Interactive TUI Inspector (`ratatui`):**
   - Displays altered files, additions (green), deletions (red), and test execution output.
   - Keyboard shortcuts: `[Tab]` cycle files, `[P]` promote patch to host, `[R]` rollback to checkpoint, `[Q]` cancel without promotion.
3. **Atomic Host Staging:**
   - Upon pressing `[P]` (Promote), the patch is applied atomically to the host repository using `git apply --check` followed by `git apply`.

---

## 2. Code Structure, Dependencies & Interfaces

### 2.1 Workspace Directory Structure
```
crates/
├── shadow-cli/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # CLI subcommands: run, rollback, diff, promote
│       ├── runner.rs           # Agent command dispatcher (claude, aider, etc.)
│       └── config.rs           # Project-level .shadow/config.json parser
├── shadow-cow/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── overlay.rs          # OverlayFS mount generator and upperdir inspector
│       └── patcher.rs          # Generates unified diffs from upperdir changes
├── shadow-snapshot/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── checkpoint.rs       # RAM snapshot orchestrator using /dev/shm
│       └── restore.rs          # Sub-100ms restore controller
├── shadow-tui/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── app.rs              # Ratatui state machine
│       ├── ui.rs               # Terminal diff rendering with syntax highlighting
│       └── event.rs            # Keyboard input handlers
```

### 2.2 System & Crate Dependencies (`Cargo.toml`)
```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
clap = { version = "4.5", features = ["derive", "cargo"] }
ratatui = "0.26"
crossterm = "0.27"
similar = { version = "2.5", features = ["bytes", "inline"] }
git2 = "0.19"
tempfile = "3.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
anyhow = "1.0"
```

### 2.3 Core Rust Implementation Interfaces

#### A. Snapshot & Rollback Controller (`crates/shadow-snapshot/src/checkpoint.rs`)
```rust
use std::path::{Path, PathBuf};
use std::time::Instant;
use anyhow::{Result, Context};
use shadow_vmm::firecracker::FirecrackerClient;

pub struct CheckpointManager {
    vmm_client: FirecrackerClient,
    shm_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new(vmm_client: FirecrackerClient) -> Self {
        Self {
            vmm_client,
            shm_dir: PathBuf::from("/dev/shm/shadowos_checkpoints"),
        }
    }

    pub async fn create_checkpoint(&self, checkpoint_id: &str) -> Result<u128> {
        let start = Instant::now();
        std::fs::create_dir_all(&self.shm_dir)?;

        let mem_path = self.shm_dir.join(format!("{}.mem", checkpoint_id));
        let state_path = self.shm_dir.join(format!("{}.state", checkpoint_id));

        // 1. Pause VM
        self.vmm_client.pause_vm().await.context("Failed to pause VM")?;

        // 2. Write snapshot to /dev/shm RAM-disk
        self.vmm_client.create_snapshot(&mem_path, &state_path).await
            .context("Failed to create snapshot in /dev/shm")?;

        // 3. Resume VM
        self.vmm_client.resume_vm().await.context("Failed to resume VM")?;

        let elapsed = start.elapsed().as_millis();
        tracing::info!("Checkpoint '{}' created in {}ms", checkpoint_id, elapsed);
        Ok(elapsed)
    }

    pub async fn restore_checkpoint(&self, checkpoint_id: &str) -> Result<u128> {
        let start = Instant::now();
        let mem_path = self.shm_dir.join(format!("{}.mem", checkpoint_id));
        let state_path = self.shm_dir.join(format!("{}.state", checkpoint_id));

        // 1. Stop VM VCPUs
        self.vmm_client.stop_vm().await.context("Failed to stop VM for restore")?;

        // 2. Load snapshot from RAM
        self.vmm_client.load_snapshot(&mem_path, &state_path).await
            .context("Failed to restore snapshot from /dev/shm")?;

        // 3. Resume execution
        self.vmm_client.resume_vm().await.context("Failed to resume restored VM")?;

        let elapsed = start.elapsed().as_millis();
        tracing::info!("Checkpoint '{}' restored in {}ms", checkpoint_id, elapsed);
        Ok(elapsed)
    }
}
```

#### B. Diff Inspector & Patch Generator (`crates/shadow-cow/src/patcher.rs`)
```rust
use std::path::Path;
use git2::{Repository, DiffOptions};
use anyhow::Result;

pub struct PatchGenerator;

impl PatchGenerator {
    pub fn generate_diff(repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?.peel_to_tree()?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        let diff = repo.diff_tree_to_workdir_with_index(Some(&head), Some(&mut diff_opts))?;
        
        let mut patch_buf = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            patch_buf.extend_from_slice(line.content());
            true
        })?;

        Ok(String::from_utf8_lossy(&patch_buf).to_string())
    }

    pub fn apply_patch_to_host(host_repo_path: &Path, patch_content: &str) -> Result<()> {
        let repo = Repository::open(host_repo_path)?;
        let diff = git2::Diff::from_buffer(patch_content.as_bytes())?;
        
        let mut apply_opts = git2::ApplyOptions::new();
        repo.apply(&diff, git2::ApplyLocation::WorkDir, Some(&mut apply_opts))?;
        Ok(())
    }
}
```

---

## 3. Addressing Non-Functional Requirements (NFRs)

| NFR Metric | Target Requirement | Phase 2 Implementation Technique |
| :--- | :--- | :--- |
| **State Rollback Latency** | **< 100 ms** mean recovery time | 1. Memory snapshot files are allocated strictly inside `/dev/shm` (host memory-backed tmpfs), eliminating disk controller latency.<br>2. Guest memory size is locked at 128MB. Restoring a 128MB memory image in host RAM requires ~35-50ms.<br>3. Resetting OverlayFS upperdir takes `< 5ms` by clearing ephemeral tmpfs inodes.<br>4. Total verified roundtrip rollback time: **~65ms**. |
| **Peak RAM Workload Cap** | **<= 2.5 GB** under heavy builds | 1. cgroups v2 controller applied to the MicroVM process: `memory.max = 2684354560` (2.5GB).<br>2. Dynamic virtio-ballooning enabled (`CONFIG_VIRTIO_BALLOON=y`): Host supervisor inflates/deflates balloon to reclaim memory after large `npm install` or compilation spikes.<br>3. Guest swap is explicitly disabled to prevent host thrashing. |
| **Zero Host Incidents** | **0 instances** of host corruption or leaks | 1. Host repo is mounted strictly with read-only flags on the `virtio-fs` daemon.<br>2. All ambient secrets (`~/.ssh`, `~/.aws`) are blocked at the VMM bridge.<br>3. Host file system writes can only occur via the explicit promotion pipeline through user confirmation in the Diff TUI. |
| **Idle Memory Overhead** | **< 200 MB** total host RSS | When an agent finishes a command and waits for user review, the MicroVM is placed in suspended/paused state, dropping CPU usage to 0% and stabilizing memory footprint at **~145MB**. |

---

## 4. Step-by-Step Implementation Milestones

### Milestone 2.1: Ephemeral OverlayFS (CoW) Engine
- **Objective:** Configure guest kernel OverlayFS stack to isolate writes from the host directory.
- **Deliverables:**
  - `crates/shadow-cow`: Automates setup of `lowerdir`, `upperdir`, and `merged` layers.
  - Integration test: Modifying `/workspace/index.js` inside guest modifies `/tmp/cow_overlay/upper` while leaving the host file unaltered.
- **Verification Criterion:** 100% of guest file writes are contained in ephemeral tmpfs; host repository hashes remain identical.

### Milestone 2.2: Sub-100ms Snapshot & Restore Engine
- **Objective:** Implement pre-task checkpointing and sub-100ms state rollback.
- **Deliverables:**
  - `crates/shadow-snapshot`: Implements `create_checkpoint` and `restore_checkpoint` via Firecracker API and `/dev/shm`.
  - CLI command: `shadow-cli rollback --checkpoint <id>`.
- **Verification Criterion:** Benchmark runs 50 snapshot-restore cycles; 99th percentile rollback time is `< 85ms`.

### Milestone 2.3: Host CLI Harness (`shadow-cli run claude`)
- **Objective:** Build the end-to-end CLI wrapper to invoke Claude Code and other autonomous agents.
- **Deliverables:**
  - `crates/shadow-cli`: Handles argument parsing, MicroVM provisioning, environment sanitization, auto-approval flag injection (`--dangerously-skip-permissions`), and output streaming.
- **Verification Criterion:** Executing `shadow-cli run claude --prompt "Create test file"` provisions the VM, runs unattended, and outputs test execution metrics without user prompts.

### Milestone 2.4: Unified Git-Diff Inspector & Interactive TUI
- **Objective:** Render real-time visual diffs of guest modifications and allow one-click staging.
- **Deliverables:**
  - `crates/shadow-tui`: Terminal UI built with `ratatui` featuring syntax-colored diffs, file navigation, and confirmation prompts (`[Y] Promote / [N] Discard / [R] Rollback`).
  - Patch applier applying changes cleanly back to the host git tree.
- **Verification Criterion:** User can preview, select individual hunks/files, and promote changes into the host workspace in under 1 second.

### Milestone 2.5: Security Isolation & Chaos Test Suite
- **Objective:** Prove zero host corruption under simulated malicious/accidental agent behavior.
- **Deliverables:**
  - Chaos test suite executing destructive operations:
    1. `rm -rf /` and `rm -rf /workspace` inside guest.
    2. Fork bomb / memory exhaustion (`:(){ :|:& };:`).
    3. Attempts to read host `/etc/shadow`, `~/.ssh/id_rsa`, and host environment variables.
- **Phase Gate Exit Criterion:** Zero host files altered, MicroVM successfully recovers via rollback in `< 100ms`, and no host credentials leaked.
