# ShadowOS: Project Plan
*A Background MicroVM & Execution Harness for Autonomous Agent Runtimes*

## 1. Executive Overview
ShadowOS is an ultra-lightweight, hardware-isolated secondary virtual operating system designed to run in the background. It provides autonomous coding agents (like Claude Code, SWE-agent, Google Antigravity IDE) with a secure, disposable sandbox. This eliminates permission fatigue and prevents host system pollution while enabling fully autonomous execution.

## 2. Release Roadmap & Implementation Phases

### Phase 1: M1 Core Engine
**Objective:** Establish the foundational virtualization boundary and execution pipeline.
*   **MicroVM Runner:** Develop a headless MicroVM runner using Firecracker for Linux (KVM) and libkrun for macOS (Apple VMM).
*   **Performance Targets:** Achieve sub-150ms provisioning time and maintain a base RAM footprint under 150MB.
*   **Filesystem Bridge:** Implement `virtio-fs` mounting to attach the host repository to the guest OS securely.
*   **Communication Layer:** Establish basic bash execution capabilities using `AF_VSOCK` for a Zero-TCP host-to-guest channel.

### Phase 2: M2 - Harness Tooling
**Objective:** Build out agent integrations, state management, and user review tools.
*   **Agent Wrapper:** Create the host CLI wrapper (e.g., `shadow-cli run claude`) to transparently mount directories, inject synthetic credentials, and run agents with auto-approve flags (e.g., `--dangerously-skip-permissions`).
*   **Host Promotion & Diff UI:** Implement the Copy-on-Write (CoW) ephemeral overlay management. Build an automated unified git-diff UI so users can review and promote guest changes back to the host filesystem.
*   **State Rollback:** Develop sub-100ms snapshot and restore checkpoints for the guest RAM and storage overlay to instantly recover from agent hallucinations or destructive actions.

### Phase 3: M3 - Ecosystem Expansion
**Objective:** Expand platform support for visual agents and deep IDE integrations.
*   **IDE Integration:** Build and deploy the Model Context Protocol (MCP) server for Google Antigravity IDE to export shadow execution primitives (`run_sandboxed_cmd`, `inspect_virtual_dom`).
*   **Headless Virtual Display:** Integrate an in-memory `Xvfb` framebuffer and headless Chromium browser to support browser-use agents without stealing the user's desktop focus.
*   **Scaling:** Implement parallel multi-agent swarming capabilities.
