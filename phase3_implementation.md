# Phase 3: M3 Ecosystem Expansion — Technical Implementation Guide
*Project ShadowOS — Background MicroVM Execution Harness for Autonomous Agent Runtimes*

---

## 1. Architectural Blueprint & System Setup

Phase 3 (M3 Ecosystem Expansion) transforms ShadowOS from a standalone CLI harness into a distributed agent execution ecosystem. It delivers three major capabilities:
1. **Model Context Protocol (MCP) Daemon Server:** Exposes sandboxed execution primitives over standard JSON-RPC (stdio/SSE) directly to the **Google Antigravity IDE**, Claude Desktop, and modern AI coding environments.
2. **Headless In-Memory Virtual Display & Browser Harness:** Embeds an in-memory `Xvfb` framebuffer and headless Chromium instance inside the MicroVM to enable autonomous visual and browser-use agents (Playwright, Puppeteer) without stealing user desktop focus or spawning disruptive GUI windows.
3. **Multi-Agent Swarm Orchestrator:** Manages pools of parallel, hardware-isolated MicroVMs running concurrent agent workers against the same base repository using branching CoW layers.

```
+-----------------------------------------------------------------------------------------+
|                                    HOST WORKSTATION                                     |
|                                                                                         |
|  +-----------------------------------------------------------------------------------+  |
|  |                  Google Antigravity IDE / Host AI Developer Tools                 |  |
|  |                                                                                   |  |
|  |  +-------------------------------------+   +-----------------------------------+  |  |
|  |  |   Antigravity Execution Controller  |   |     Artifacts & Manager View      |  |  |
|  |  |   (Calls MCP Tools via stdio)       |   |  (Terminal, Playwright WebP, DOM) |  |  |
|  |  +-------------------------------------+   +-----------------------------------+  |  |
|  +-----------------------------------|-----------------------------------------------+  |
+--------------------------------------|--------------------------------------------------+
                                       │ Stdio / JSON-RPC 2.0 (MCP Protocol)
+--------------------------------------|--------------------------------------------------+
|                                      ▼                                                  |
|  +-----------------------------------------------------------------------------------+  |
|  |                         ShadowOS MCP Daemon Server (Rust)                         |  |
|  |                                                                                   |  |
|  |  Exported Primitives:                                                             |  |
|  |    - run_sandboxed_cmd(command, timeout_ms, workdir)                              |  |
|  |    - inspect_virtual_dom(selector, format)                                        |  |
|  |    - capture_browser_screenshot(full_page)                                        |  |
|  |    - rollback_checkpoint(checkpoint_id)                                           |  |
|  |    - promote_changes(hunk_filter)                                                 |  |
|  |                                                                                   |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  |                   Swarm Orchestrator & VM Pool Controller                   |  |  |
|  |  |        (Pool size: 1-8 VMs, Port mapping, CID multiplexer, CPU Pinning)      |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  +-------------------|------------------------------------------|--------------------+  |
+----------------------|------------------------------------------|-----------------------+
                       │ AF_VSOCK Pool Channel                    │
+----------------------|------------------------------------------|-----------------------+
|                      ▼ (VM 1: Agent Worker)                     ▼ (VM 2: Tester Worker) |
|  +---------------------------------------------+  +----------------------------------+  |
|  |          SHADOW GUEST OS (MicroVM 1)        |  |    SHADOW GUEST OS (MicroVM 2)   |  |
|  |                                             |  |                                  |  |
|  |  +---------------------------------------+  |  |  +----------------------------+  |  |
|  |  | OverlayFS: upperdir_vm1 (Isolated CoW)|  |  |  | OverlayFS: upperdir_vm2    |  |  |
|  |  +---------------------------------------+  |  |  +----------------------------+  |  |
|  |  | In-Memory Virtual Display:            |  |  |  | Build & Unit Test Runner   |  |  |
|  |  |   - Xvfb :99 -screen 0 1280x720x16    |  |  |  | npm test / pytest          |  |  |
|  |  |     (Allocated in /dev/shm, ~1.8MB)   |  |  |  +----------------------------+  |  |
|  |  |   - Headless Chromium with CDP Engine |  |  +----------------------------------+  |
|  |  |     (Listening on internal port 9222) |  |                                        |
|  |  +---------------------------------------+  |                                        |
|  |  | Playwright & Browser-Use Agent Runtime|  |                                        |
|  +---------------------------------------------+                                        |
+-----------------------------------------------------------------------------------------+
```

---

### 1.1 Antigravity IDE & Model Context Protocol (MCP) Daemon Bridge

The ShadowOS MCP daemon runs as a background service on the host. When Antigravity IDE starts, it spawns `shadow-mcp` via stdio and discovers the available tools.

#### JSON-RPC MCP Schema & Tool Definitions

1. `run_sandboxed_cmd`:
   Executes commands inside the MicroVM, streaming stdout/stderr chunks back through MCP notifications.
   ```json
   {
     "name": "run_sandboxed_cmd",
     "description": "Executes a shell command inside the hardware-isolated ShadowOS MicroVM",
     "inputSchema": {
       "type": "object",
       "properties": {
         "command": { "type": "string", "description": "Command string to run" },
         "timeout_seconds": { "type": "integer", "default": 120 },
         "auto_rollback_on_failure": { "type": "boolean", "default": true }
       },
       "required": ["command"]
     }
   }
   ```

2. `inspect_virtual_dom`:
   Interrogates the in-guest headless Chromium instance to retrieve cleaned DOM tree representations for visual agent comprehension.
   ```json
   {
     "name": "inspect_virtual_dom",
     "description": "Inspects the DOM tree of the sandboxed Chromium browser running inside ShadowOS",
     "inputSchema": {
       "type": "object",
       "properties": {
         "selector": { "type": "string", "default": "body" },
         "include_bounding_boxes": { "type": "boolean", "default": true }
       }
     }
   }
   ```

3. `capture_browser_screenshot`:
   Captures frame buffer snapshots directly from the in-guest `Xvfb` display without window creation, encoding output as an image artifact for the Antigravity Manager View.

---

### 1.2 In-Memory Headless Virtual Display & Chromium CDP Bridge

Running browser-use agents on bare-metal frequently disrupts developer focus (browser windows pop over IDE windows, take mouse focus, or crash). ShadowOS eliminates this:

1. **In-Memory Framebuffer (`Xvfb` / `cage`):**
   - Inside the guest microVM, `Xvfb` is started targeting `/dev/shm`:
     ```bash
     export DISPLAY=:99
     Xvfb :99 -screen 0 1280x720x16 -shmem &
     ```
   - At 1280x720 with 16-bit color depth, the entire framebuffer consumes only **1.84MB** of RAM.
2. **Chromium DevTools Protocol (CDP) Bridge:**
   - Chromium starts with hardware acceleration disabled and CDP bound to localhost:
     ```bash
     chromium-browser \
       --remote-debugging-port=9222 \
       --disable-gpu \
       --no-sandbox \
       --disable-dev-shm-usage \
       --window-size=1280,720 \
       --display=:99 &
     ```
3. **CDP Over VSOCK Proxy:**
   - A lightweight forwarder (`shadow-cdp-proxy`) maps guest TCP port `9222` to `AF_VSOCK` port `5002`.
   - The host MCP daemon can connect Playwright or Puppeteer directly to the guest Chromium over virtual sockets without opening any host network ports.

---

### 1.3 Multi-Agent Swarming & Resource Management

For workflows where multiple agents operate simultaneously (e.g., SWE-agent running test generation in parallel with Claude Code refactoring backend code):

1. **Branching Overlay Architecture:**
   - All MicroVM instances share the exact same read-only base repository via `virtio-fs`.
   - Each VM instance `k` receives an independent ephemeral upper directory:
     `/tmp/cow_vm_01/upper`, `/tmp/cow_vm_02/upper`, ..., `/tmp/cow_vm_0N/upper`.
2. **CPU Pinning & Resource Quotas:**
   - Linux `cgroups v2` and `taskset` isolate agent workloads:
     ```bash
     # Pin VM 1 to cores 0-1, cap RAM at 1GB
     echo "0-1" > /sys/fs/cgroup/shadow/vm1/cpuset.cpus
     echo "1073741824" > /sys/fs/cgroup/shadow/vm1/memory.max
     ```
3. **Multi-Agent Merge Conflict Resolver:**
   - Before promoting changes from swarm workers, the host manager performs a 3-way git merge (`git merge-tree`) across the diffs emitted by `vm1` and `vm2`.

---

## 2. Code Structure, Dependencies & Interfaces

### 2.1 Workspace Directory Structure
```
crates/
├── shadow-mcp/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # MCP stdio JSON-RPC server binary
│       ├── router.rs           # Tool dispatcher (run_cmd, inspect_dom, etc.)
│       ├── tools/
│       │   ├── exec.rs         # Maps to shadow-vsock execution
│       │   ├── browser.rs      # CDP client for DOM and screenshots
│       │   └── promotion.rs    # Checkpoint and patch promotion tools
│       └── artifacts.rs        # Streams files to Antigravity Manager View
├── shadow-display/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── xvfb.rs             # Guest Xvfb supervisor
│       └── cdp_bridge.rs       # VSOCK-to-CDP socket multiplexer
├── shadow-swarm/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── pool.rs             # MicroVM pool allocator & lifecycle manager
│       ├── scheduler.rs        # Worker task queue & core affinity assigner
│       └── merge.rs            # Multi-agent Git patch branch & merge resolver
```

### 2.2 System & Crate Dependencies (`Cargo.toml`)
```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
tokio-vsock = "0.4"
tracing = "0.1"
anyhow = "1.0"
headless_chrome = { version = "1.0", default-features = false }
image = { version = "0.25", features = ["png", "webp"] }
```

### 2.3 Core Rust Implementation Interfaces

#### A. MCP Server Handler (`crates/shadow-mcp/src/router.rs`)
```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::{Result, bail};
use shadow_swarm::pool::VmPool;

#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<Value>,
    pub error: Option<Value>,
}

pub struct McpServer {
    pool: VmPool,
}

impl McpServer {
    pub fn new(pool: VmPool) -> Self {
        Self { pool }
    }

    pub async fn handle_tool_call(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        match tool_name {
            "run_sandboxed_cmd" => {
                let cmd = arguments["command"].as_str().unwrap_or_default();
                let vm = self.pool.acquire_vm().await?;
                let output = vm.execute_bash(cmd).await?;
                self.pool.release_vm(vm).await?;
                Ok(serde_json::json!({ "stdout": output.stdout, "exit_code": output.exit_code }))
            }
            "inspect_virtual_dom" => {
                let selector = arguments["selector"].as_str().unwrap_or("body");
                let vm = self.pool.acquire_vm().await?;
                let dom_json = vm.query_cdp_dom(selector).await?;
                self.pool.release_vm(vm).await?;
                Ok(serde_json::json!({ "dom": dom_json }))
            }
            "capture_browser_screenshot" => {
                let vm = self.pool.acquire_vm().await?;
                let png_bytes = vm.capture_framebuffer().await?;
                self.pool.release_vm(vm).await?;
                let b64 = base64::encode(png_bytes);
                Ok(serde_json::json!({ "mime_type": "image/png", "data": b64 }))
            }
            _ => bail!("Unknown tool: {}", tool_name),
        }
    }
}
```

#### B. Swarm VM Pool Manager (`crates/shadow-swarm/src/pool.rs`)
```rust
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use shadow_vmm::traits::VMMDriver;
use anyhow::Result;

pub struct SwarmInstance {
    pub instance_id: u32,
    pub guest_cid: u32,
    pub upperdir_path: std::path::PathBuf,
}

pub struct VmPool {
    available_instances: Arc<Mutex<Vec<SwarmInstance>>>,
    semaphore: Arc<Semaphore>,
}

impl VmPool {
    pub fn new(max_concurrent: usize, instances: Vec<SwarmInstance>) -> Self {
        Self {
            available_instances: Arc::new(Mutex::new(instances)),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn acquire_vm(&self) -> Result<SwarmInstance> {
        let _permit = self.semaphore.acquire().await?;
        let mut list = self.available_instances.lock().await;
        list.pop().ok_or_else(|| anyhow::anyhow!("No VM available in pool"))
    }

    pub async fn release_vm(&self, instance: SwarmInstance) -> Result<()> {
        let mut list = self.available_instances.lock().await;
        list.push(instance);
        Ok(())
    }
}
```

---

## 3. Addressing Non-Functional Requirements (NFRs)

| NFR Metric | Target Requirement | Phase 3 Implementation Technique |
| :--- | :--- | :--- |
| **Idle Memory Overhead** | **< 200 MB** base per instance | 1. `Xvfb` frame buffer allocated dynamically in `/dev/shm` (only 1.8MB for 1280x720x16).<br>2. Chromium processes run with `--disable-gpu` and `--js-flags="--max-old-space-size=128"` to suppress V8 memory bloat.<br>3. Swarm pool maintains idle standby VMs in paused state, reducing idle CPU usage to 0% and RSS to ~145MB. |
| **Zero Desktop Focus Stealing** | **0 instances** of window stealing | 1. MicroVM runs headless with no X11/Wayland bridge to the host desktop environment.<br>2. All graphical operations target the virtual `Xvfb` display (`:99`).<br>3. Browser automation takes place inside the guest sandbox; developer host screen remains 100% undisturbed. |
| **Wall-Clock Task Completion** | **> 3.5x reduction** in task time | 1. Swarm parallelization splits coding subtasks across independent MicroVMs (e.g. parallel linting, compiling, and testing).<br>2. Eliminates human confirmation prompts across long test suites and build iterations.<br>3. Fast branch switching enabled by instant CoW upperdir creation (<10ms). |
| **Artifact Stream Latency** | **< 50 ms** image/stream transfer | 1. Framebuffer snapshots and Playwright WebP chunks are transferred over `AF_VSOCK` via direct memory buffers.<br>2. MCP server streams events directly to the Antigravity IDE UI via stdio chunks. |

---

## 4. Step-by-Step Implementation Milestones

### Milestone 3.1: Model Context Protocol (MCP) Daemon Server
- **Objective:** Build and deploy the compliant MCP server exposing ShadowOS execution primitives.
- **Deliverables:**
  - `crates/shadow-mcp`: Stdio JSON-RPC 2.0 server.
  - Implements `run_sandboxed_cmd`, `rollback_checkpoint`, `promote_changes`.
  - Integration with Antigravity IDE: Confirmed discovery and tool execution from IDE agent sessions.
- **Verification Criterion:** Antigravity IDE triggers `run_sandboxed_cmd` over stdio and receives streaming outputs without errors.

### Milestone 3.2: Headless Virtual Display & Chromium CDP Engine
- **Objective:** Set up in-guest `Xvfb` and headless Chromium with DevTools Protocol over vsock.
- **Deliverables:**
  - In-guest daemon launches `Xvfb :99` in `/dev/shm` and starts Chromium with `--remote-debugging-port=9222`.
  - `crates/shadow-display`: Proxies CDP commands from host over `AF_VSOCK` port `5002`.
- **Verification Criterion:** Playwright script executes headless navigation inside guest and captures screenshot without opening any windows on host screen.

### Milestone 3.3: Visual Agent & DOM Inspection Tooling
- **Objective:** Export `inspect_virtual_dom` and `capture_browser_screenshot` to the MCP server.
- **Deliverables:**
  - MCP tool returns clean JSON DOM structures containing accessibility tags and coordinates.
  - Generates WebP image artifacts streamed directly into Antigravity Manager View.
- **Verification Criterion:** Autonomous browser-use agent can navigate a multi-page web app, detect buttons, and click elements purely via MCP tools.

### Milestone 3.4: Multi-Agent Swarm Coordinator
- **Objective:** Orchestrate up to 8 parallel MicroVM workers with branching CoW layers.
- **Deliverables:**
  - `crates/shadow-swarm`: Pool manager maintaining warm microVMs, dynamically mounting individual `upperdir` layers.
  - Concurrency governor enforcing CPU pinning and 2.5GB peak memory caps.
  - 3-way Git merge conflict detector for swarm results.
- **Verification Criterion:** 4 parallel agents run concurrent test suites against the same project repository without file clashes or memory spikes.

### Milestone 3.5: End-to-End Autonomous Coding Benchmark
- **Objective:** Execute an unattended multi-hour coding benchmark verifying full Phase 1-3 integration.
- **Deliverables:**
  - Benchmark scenario: Full repository migration with automated test suite and Playwright end-to-end browser tests run completely unattended.
  - Metrics validation:
    - > 3.5x reduction in wall-clock time vs manual human-prompted runs.
    - Zero host file corruption or leaks.
    - Mean rollback latency `< 100ms`.
- **Phase Gate Exit Criterion:** Complete autonomous execution of 20 complex coding tasks with 100% host isolation and zero human intervention during sandboxed steps.
