# Phase 1: M1 Core Engine — Technical Implementation Guide
*Project ShadowOS — Hardware-Isolated Background MicroVM Execution Harness*

---

## 1. Architectural Blueprint & System Setup

Phase 1 (M1 Core Engine) establishes the foundational virtualization boundary, the kernel/rootfs runtime, the host-to-guest shared filesystem bridge, and the zero-TCP IPC channel. 

The primary architectural requirement is cross-platform hypervisor abstraction:
- **Linux (x86_64 / aarch64):** Hardware-assisted virtualization via KVM (`/dev/kvm`), managed through **Firecracker** with Linux jailer isolation (cgroups v2, seccomp, PID/network namespaces).
- **macOS (Apple Silicon - aarch64):** Hardware-accelerated hypervisor virtualization via Apple's `Virtualization.framework` / `Hypervisor.framework`, abstracted using **libkrun** and **libkrunfw**.

```
+-----------------------------------------------------------------------------------------+
|                                HOST OPERATING SYSTEM                                    |
|                                                                                         |
|  +-----------------------------------------------------------------------------------+  |
|  |                           ShadowOS Host Runtime (Rust)                           |  |
|  |                                                                                   |  |
|  |  +-----------------------------------+     +-----------------------------------+  |  |
|  |  |      Firecracker Driver (KVM)     |     |       libkrun Driver (Apple VMM)  |  |  |
|  |  +-----------------------------------+     +-----------------------------------+  |  |
|  |                   |                                          |                    |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  |             virtiofsd Daemon (Rust)  <--->  Host Working Directory          |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |                   |                                          |                    |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  |             AF_VSOCK Host Listener / Multiplexer (CID 2 <-> CID 3)          |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  +-------------------|------------------------------------------|--------------------+  |
+----------------------|------------------------------------------|-----------------------+
                       | Hardware Virtualization Boundary         |
                       | (KVM ioctls / Apple Hypervisor API)      |
+----------------------|------------------------------------------|-----------------------+
|                      ▼                                          ▼                       |
|  +-----------------------------------------------------------------------------------+  |
|  |                               SHADOW GUEST OS (MicroVM)                           |  |
|  |                                                                                   |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  | Minimal Uncompressed Kernel (vmlinux, ~4.1MB, stripped ACPI/PCI/Sound/USB)  |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  | Alpine-based Tiny Rootfs (initramfs, ~18MB, Musl Libc, Busybox, Busybox sh) |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  |  | shadow-guest-init (PID 1) & shadow-guest-agent                               |  |  |
|  |  |   - Mounts /dev, /proc, /sys, /tmp (tmpfs)                                  |  |  |
|  |  |   - Mounts virtio-fs tag 'shadow-workspace' to /workspace                   |  |  |
|  |  |   - Binds AF_VSOCK (Port 5001) for Command Execution Engine                 |  |  |
|  |  +-----------------------------------------------------------------------------+  |  |
|  +-----------------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------------+
```

---

### 1.1 Hypervisor Configuration & Provisioning Pipeline

#### A. Linux: Firecracker + Jailer Setup
1. **Host Prerequisites:**
   - Linux kernel 5.10+ with `CONFIG_KVM=m`, `CONFIG_KVM_INTEL=m` or `CONFIG_KVM_AMD=m`.
   - Access to `/dev/kvm` (permissions: mode `0660`, group `kvm`).
   - cgroups v2 mounted at `/sys/fs/cgroup`.
2. **Jailer Invocation:**
   Firecracker is executed via the `jailer` binary to enforce zero ambient authority:
   ```bash
   jailer --id vm_01jk98 \
          --exec-file /usr/local/bin/firecracker \
          --uid 10001 --gid 10001 \
          --chroot-base-dir /srv/shadowos/jail \
          --netns /var/run/netns/shadow_empty \
          --daemonize -- \
          --api-sock /run/firecracker.socket
   ```
3. **VM Configuration over Unix Domain Socket (UDS):**
   - Boot source: uncompressed `vmlinux`, `boot_args`: `console=ttyS0 reboot=k panic=1 pci=off nomodules quiet init=/init`.
   - Machine config: `vcpu_count: 2`, `mem_size_mib: 128`, `ht_enabled: false`.
   - Drives: Ephemeral rootfs attached via `virtio-block` or mounted read-only base rootfs.

#### B. macOS: libkrun / libkrunfw Setup
1. **Host Prerequisites:**
   - macOS 14.0+ (Sonoma) on Apple Silicon (M1/M2/M3/M4).
   - Entitlements: `com.apple.security.hypervisor` signed on the host runner executable.
2. **libkrun Configuration:**
   - Initializes a microVM directly in-process without spawning a separate heavyweight QEMU emulator.
   - Configured via C/Rust FFI bindings:
     ```rust
     krun_set_vm_config(ctx, num_vcpus, ram_mib);
     krun_set_root(ctx, rootfs_path);
     krun_add_vsock_port(ctx, 5001, host_listen_port);
     krun_add_virtiofs(ctx, "shadow-workspace", host_workdir_path);
     krun_start_enter(ctx);
     ```

---

### 1.2 Kernel & Rootfs Build Specifications

#### Stripped Linux Kernel (`vmlinux`)
- Standard distributions ship bloated kernels (30MB+ compressed, 80MB uncompressed) loading hundreds of kernel modules. ShadowOS compiles an ultra-lean static kernel:
  - **Base Source:** Stable Linux LTS (6.6.x).
  - **Disabled Subsystems:** `CONFIG_NET` (socket networking disabled or restricted to loopback/vsock), `CONFIG_PCI=n`, `CONFIG_ACPI=n`, `CONFIG_SOUND=n`, `CONFIG_WIRELESS=n`, `CONFIG_USB=n`, `CONFIG_DRM=n`.
  - **Enabled Subsystems (Built-in `=y`):**
    - `CONFIG_VIRTIO=y`
    - `CONFIG_VIRTIO_MMIO=y` (for Firecracker)
    - `CONFIG_VIRTIO_PCI=y` (for libkrun)
    - `CONFIG_VIRTIO_FS=y`
    - `CONFIG_VHOST_VSOCK=y` & `CONFIG_VIRTIO_VSOCK=y`
    - `CONFIG_OVERLAY_FS=y`
    - `CONFIG_TMPFS=y`, `CONFIG_TMPFS_POSIX_ACL=y`
    - `CONFIG_BINFMT_ELF=y`, `CONFIG_BINFMT_SCRIPT=y`
    - `CONFIG_FUSE_FS=y`
  - **Output Size:** Uncompressed `vmlinux` < 4.5MB. Kernel boot time to `/init`: **~18ms**.

#### Minimal Alpine Rootfs (`rootfs.ext4` / `initramfs.cpio.gz`)
- Built from minimal Alpine Linux rootfs tarball (`alpine-minirootfs-3.20-aarch64/x86_64`).
- Stripped of APK caches, documentation, unused charsets, terminfo.
- Installed utilities:
  - BusyBox statically linked (core utilities: `sh`, `ls`, `cat`, `grep`, `awk`, `mkdir`, `mount`, etc.).
  - Node.js LTS (statically linked or musl minimal) and Python 3 minimal runtime.
  - `shadow-guest-agent`: Statically linked Rust binary placed at `/sbin/shadow-guest-agent`.
- Custom `/init` shell/binary script:
  ```sh
  #!/bin/sh
  mount -t proc none /proc
  mount -t sysfs none /sys
  mount -t devtmpfs none /dev
  mkdir -p /dev/pts /dev/shm
  mount -t devpts none /dev/pts
  mount -t tmpfs none /dev/shm
  mount -t tmpfs none /tmp

  # Mount virtio-fs host workspace
  mkdir -p /workspace
  mount -t virtiofs shadow-workspace /workspace

  # Hand off to guest execution agent
  exec /sbin/shadow-guest-agent
  ```

---

### 1.3 Filesystem Bridge: `virtio-fs` Architecture

To achieve direct workspace interaction without TCP networking or Samba/NFS overhead:
1. **Host Daemon (`virtiofsd`):**
   - Implemented using the standalone Rust `virtiofsd` binary (`/usr/libexec/virtiofsd`).
   - Launch parameters:
     ```bash
     virtiofsd \
       --socket-path=/run/virtiofsd_vm01.sock \
       --shared-dir=/path/to/host/repo \
       --cache=always \
       --sandbox=chroot \
       --thread-pool-size=4 \
       --announce-submounts
     ```
2. **Firecracker Integration:**
   - Firecracker connects to the `virtiofsd` UDS via a vhost-user-fs device configuration.
3. **Guest Mount:**
   - Inside the guest: `mount -t virtiofs shadow-workspace /workspace`.
   - All I/O operations are serviced directly by shared memory / DMA queues through the vhost-user device.

---

### 1.4 Zero-TCP Communication Protocol: `AF_VSOCK`

To prevent network exposure and eliminate TCP stack latency, communication between the host supervisor and guest agent uses virtual sockets (`AF_VSOCK`):
- **Host Context ID (CID):** `VMADDR_CID_HOST = 2`
- **Guest Context ID (CID):** Configured dynamically per VM (e.g., `CID = 3`, `4`, `...`).
- **Reserved Port:** `5001` (ShadowOS Agent Protocol).

#### Framing Protocol (`ShadowFrame`)
The raw stream over `AF_VSOCK` is framed using a length-prefixed binary protocol with JSON payloads and raw binary stream multiplexing:

```
+---------------+----------------+--------------------+-----------------------+
| Magic (2B)    | MsgType (1B)   | StreamID (1B)      | PayloadLength (4B)    |
| 0x53 0x4F     | 0x01 = CMD_REQ | 0x01 = STDIN       | Big-Endian uint32     |
| ("SO")        | 0x02 = STDOUT  | 0x02 = STDOUT      |                       |
|               | 0x03 = STDERR  | 0x03 = STDERR      |                       |
|               | 0x04 = EXIT    | 0x00 = CTRL        |                       |
|               | 0x05 = HEARTBT |                    |                       |
+---------------+----------------+--------------------+-----------------------+
|                           Payload Data (N Bytes)                            |
+-----------------------------------------------------------------------------+
```

---

## 2. Code Structure, Dependencies & Interfaces

### 2.1 Directory Structure
```
crates/
├── shadow-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       └── protocol.rs         # AF_VSOCK framing, packet serialization
├── shadow-vmm/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs           # VMMDriver trait (spawn, pause, resume, teardown)
│       ├── firecracker.rs      # Firecracker UDS REST client & jailer spawner
│       ├── libkrun.rs          # macOS libkrun FFI driver
│       └── virtiofs.rs         # virtiofsd process supervisor
├── shadow-vsock/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── host_stream.rs      # Host-side multiplexer (stdin, stdout, stderr, exit)
│       └── channel.rs          # Tokio mpsc channel bridge
├── shadow-guest-agent/
│   ├── Cargo.toml              # Statically compiled binary (x86_64/aarch64-unknown-linux-musl)
│   └── src/
│       ├── main.rs             # PID 1 or daemon entry point
│       ├── vsock_server.rs     # Binds AF_VSOCK port 5001
│       └── exec.rs             # Spawns /bin/sh, attaches pipes to vsock multiplexer
scripts/
├── build-kernel.sh             # Compiles stripped vmlinux 6.6
└── build-rootfs.sh             # Assembles Alpine-based minimal rootfs
```

### 2.2 System & Rust Crate Dependencies

#### Host Crate Dependencies (`Cargo.toml`)
```toml
[dependencies]
tokio = { version = "1.38", features = ["full"] }
tokio-vsock = "0.4"
nix = { version = "0.29", features = ["process", "signal", "mount", "fs"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
hyper = { version = "1.4", features = ["client", "http1"] }
hyperlocal = "0.9"           # Unix Domain Socket transport for Firecracker HTTP API
bytes = "1.6"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

#### Guest Agent Dependencies (`crates/shadow-guest-agent/Cargo.toml`)
```toml
[package]
name = "shadow-guest-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.38", default-features = false, features = ["rt", "process", "io-util", "net", "sync", "macros"] }
tokio-vsock = "0.4"
bytes = "1.6"
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde_json = "1.0"
```

### 2.3 Core Rust Implementation Interfaces

#### `VMMDriver` Trait (`crates/shadow-vmm/src/traits.rs`)
```rust
use async_trait::async_trait;
use std::path::PathBuf;
use shadow_core::error::Result;

pub struct VmConfig {
    pub vm_id: String,
    pub vcpu_count: u8,
    pub memory_size_mib: u32,
    pub kernel_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub shared_dir: PathBuf,
    pub guest_cid: u32,
}

#[async_trait]
pub trait VMMDriver: Send + Sync {
    async fn init(&mut self, config: &VmConfig) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn pause(&mut self) -> Result<()>;
    async fn resume(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn get_vsock_fd(&self) -> Result<i32>;
}
```

#### Guest Command Executor (`crates/shadow-guest-agent/src/exec.rs`)
```rust
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn execute_command(
    cmd: String,
    args: Vec<String>,
    mut stdin_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    stdout_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    stderr_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
) -> Result<i32, std::io::Error> {
    let mut child = Command::new(&cmd)
        .args(&args)
        .current_dir("/workspace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut child_stdin = child.stdin.take().unwrap();
    let mut child_stdout = child.stdout.take().unwrap();
    let mut child_stderr = child.stderr.take().unwrap();

    // Pipe stdin
    tokio::spawn(async move {
        while let Some(chunk) = stdin_rx.recv().await {
            if child_stdin.write_all(&chunk).await.is_err() {
                break;
            }
        }
    });

    // Pipe stdout
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        while let Ok(n) = child_stdout.read(&mut buf).await {
            if n == 0 { break; }
            let _ = stdout_tx.send(buf[..n].to_vec()).await;
        }
    });

    // Pipe stderr
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        while let Ok(n) = child_stderr.read(&mut buf).await {
            if n == 0 { break; }
            let _ = stderr_tx.send(buf[..n].to_vec()).await;
        }
    });

    let status = child.wait().await?;
    Ok(status.code().unwrap_or(-1))
}
```

---

## 3. Addressing Non-Functional Requirements (NFRs)

| NFR Metric | Target Requirement | Phase 1 Implementation Technique |
| :--- | :--- | :--- |
| **MicroVM Provisioning Time** | **< 150 ms** cold boot | 1. Direct uncompressed `vmlinux` boot (skips decompressor, saves ~35ms).<br>2. Kernel command line: `quiet pci=off nomodules lpj=100000 init=/sbin/shadow-guest-agent` (skips calibration, module probing, and shell init script).<br>3. Static memory allocation (`MAP_SHARED | MAP_ANONYMOUS`) with no swap paging.<br>4. MicroVM warm pool: Pre-spawns 1 idle VM suspended in memory. |
| **Base RAM Footprint** | **< 150 MB** active | 1. Kernel footprint: Stripped static kernel uses only **14MB** kernel heap.<br>2. Guest userspace: Alpine Musl runtime + BusyBox + statically linked Rust guest agent consumes **18MB** RSS.<br>3. Total guest allocated memory: Configured strictly at **128MB** via VMM config.<br>4. Host VMM overhead: Firecracker process RSS is **< 12MB**; libkrun overhead is **< 8MB**. Total host + guest idle footprint: **~145MB**. |
| **Security Boundary** | Hardware isolation (Ring -1) | 1. Linux KVM / Apple Hypervisor hardware virtualization guarantees that even root exploit inside guest cannot access host RAM or CPU registers.<br>2. Firecracker Jailer enforces seccomp filter dropping 180+ syscalls.<br>3. Zero TCP/IP networking (host network interfaces completely hidden from guest).<br>4. No ambient environment variables passed from host to guest. |
| **I/O Performance** | **>= 85%** native NVMe | 1. `virtiofsd` configured with Direct Access (DAX) mapping (`--cache=always`), mapping host page caches directly into guest physical memory address space.<br>2. Avoids guest page cache replication and redundant copy operations.<br>3. MicroVM internal `/tmp` uses pure in-memory `tmpfs` achieving >4GB/s bandwidth for build artifacts. |

---

## 4. Step-by-Step Implementation Milestones

### Milestone 1.1: Stripped Kernel & Minimal Rootfs Toolchain
- **Objective:** Build reproducible artifacts for `vmlinux` and `rootfs.ext4`.
- **Deliverables:**
  - Automated bash/docker script `scripts/build-kernel.sh` producing `vmlinux` (size < 4.5MB).
  - Automated script `scripts/build-rootfs.sh` downloading Alpine minirootfs, bundling `shadow-guest-agent`, and emitting `rootfs.ext4` (size < 25MB).
- **Verification Criterion:** Kernel boots in QEMU/Firecracker to guest shell in `< 30ms`.

### Milestone 1.2: Cross-Platform VMM Driver Engine
- **Objective:** Abstract Firecracker (Linux) and libkrun (macOS) behind the `VMMDriver` trait.
- **Deliverables:**
  - `crates/shadow-vmm/src/firecracker.rs`: Manages jailer lifecycle, UDS socket API calls, microVM spawn and teardown.
  - `crates/shadow-vmm/src/libkrun.rs`: FFI bindings to `libkrun` for macOS Sonoma.
- **Verification Criterion:** Spawning an empty microVM and receiving heartbeats completes in `< 120ms` on both platforms.

### Milestone 1.3: virtio-fs Host-to-Guest Shared Filesystem
- **Objective:** Mount host project repository into `/workspace` inside the MicroVM.
- **Deliverables:**
  - `crates/shadow-vmm/src/virtiofs.rs`: Manages `virtiofsd` subprocess lifecycle and socket descriptors.
  - Guest `/init` script mounts `virtiofs shadow-workspace /workspace`.
- **Verification Criterion:** Host file creations, modifications, and reads are immediately accessible inside `/workspace` with I/O benchmark reaching >=85% native NVMe throughput.

### Milestone 1.4: AF_VSOCK Host-to-Guest Bash Execution Engine
- **Objective:** Stream stdin, stdout, stderr, and exit codes between host and guest over AF_VSOCK.
- **Deliverables:**
  - `crates/shadow-core/src/protocol.rs`: Binary packet framing implementation.
  - `crates/shadow-guest-agent`: Listening on CID 3, Port 5001, executing requested binaries (`/bin/sh`, `npm`, `python3`).
  - Host runner CLI test: `cargo run -p shadow-vmm -- test-exec "ls -la /workspace"`.
- **Verification Criterion:** Bash commands execute reliably with real-time stream output and correct exit code propagation.

### Milestone 1.5: Benchmark & NFR Validation Suite
- **Objective:** Validate all Phase 1 PRD metrics through automated benchmarks.
- **Deliverables:**
  - Criterion benchmark harness testing:
    1. Cold-start provisioning latency (target: `< 150ms`).
    2. Idle RAM consumption via `smem` / `ps_mem` (target: `< 150MB`).
    3. File I/O throughput via `fio` comparing host vs virtio-fs (target: `>= 85%`).
- **Phase Gate Exit Criterion:** 100 consecutive boot-execute-teardown cycles complete with 0 failures, mean boot time `< 135ms`, and peak idle RSS `< 148MB`.
