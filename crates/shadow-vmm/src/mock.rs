use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use serde_json::Value;
use shadow_core::{Result, ShadowError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockVmState {
    Unconfigured,
    Configured,
    Running,
    Paused,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub method: String,
    pub endpoint: String,
    pub payload: Option<Value>,
    pub timestamp: Instant,
}

#[derive(Clone)]
pub struct MockHypervisor {
    pub state: Arc<Mutex<MockVmState>>,
    pub recorded_calls: Arc<Mutex<Vec<RecordedCall>>>,
    pub memory_size_mib: Arc<Mutex<u32>>,
    pub vcpu_count: Arc<Mutex<u8>>,
    pub active_snapshots: Arc<Mutex<HashMap<String, String>>>, // mem_path -> state_path
}

impl Default for MockHypervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHypervisor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockVmState::Unconfigured)),
            recorded_calls: Arc::new(Mutex::new(Vec::new())),
            memory_size_mib: Arc::new(Mutex::new(0)),
            vcpu_count: Arc::new(Mutex::new(0)),
            active_snapshots: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Handles an incoming Firecracker UDS REST API call
    pub fn handle_request(&self, method: &str, endpoint: &str, body: Option<Value>) -> Result<Value> {
        let mut calls = self.recorded_calls.lock().unwrap();
        calls.push(RecordedCall {
            method: method.to_string(),
            endpoint: endpoint.to_string(),
            payload: body.clone(),
            timestamp: Instant::now(),
        });

        match (method, endpoint) {
            ("PUT", "/boot-source") => {
                let mut state = self.state.lock().unwrap();
                if *state == MockVmState::Unconfigured {
                    *state = MockVmState::Configured;
                }
                Ok(serde_json::json!({ "status": "boot-source configured" }))
            }
            ("PUT", "/machine-config") => {
                if let Some(ref b) = body {
                    if let Some(mem) = b.get("mem_size_mib").and_then(|v| v.as_u64()) {
                        *self.memory_size_mib.lock().unwrap() = mem as u32;
                    }
                    if let Some(vcpu) = b.get("vcpu_count").and_then(|v| v.as_u64()) {
                        *self.vcpu_count.lock().unwrap() = vcpu as u8;
                    }
                }
                Ok(serde_json::json!({ "status": "machine-config configured" }))
            }
            ("PUT", "/drives/rootfs") => {
                Ok(serde_json::json!({ "status": "drive attached" }))
            }
            ("PUT", "/vsock") => {
                Ok(serde_json::json!({ "status": "vsock device attached" }))
            }
            ("PUT", "/actions") => {
                if let Some(ref b) = body {
                    match b.get("action_type").and_then(|v| v.as_str()) {
                        Some("InstanceStart") => {
                            let mut state = self.state.lock().unwrap();
                            *state = MockVmState::Running;
                            Ok(serde_json::json!({ "status": "instance started" }))
                        }
                        Some("SendCtrlAltDel") => {
                            let mut state = self.state.lock().unwrap();
                            *state = MockVmState::Terminated;
                            Ok(serde_json::json!({ "status": "termination initiated" }))
                        }
                        other => Err(ShadowError::Vmm(format!("Unknown action: {:?}", other))),
                    }
                } else {
                    Err(ShadowError::Vmm("Missing action body".into()))
                }
            }
            ("PATCH", "/vm") => {
                if let Some(ref b) = body {
                    match b.get("state").and_then(|v| v.as_str()) {
                        Some("Paused") => {
                            let mut state = self.state.lock().unwrap();
                            *state = MockVmState::Paused;
                            Ok(serde_json::json!({ "status": "vm paused" }))
                        }
                        Some("Resumed") => {
                            let mut state = self.state.lock().unwrap();
                            *state = MockVmState::Running;
                            Ok(serde_json::json!({ "status": "vm resumed" }))
                        }
                        other => Err(ShadowError::Vmm(format!("Invalid vm state transition: {:?}", other))),
                    }
                } else {
                    Err(ShadowError::Vmm("Missing vm patch body".into()))
                }
            }
            ("PUT", "/snapshot/create") => {
                if let Some(ref b) = body {
                    let mem_path = b.get("mem_file_path").and_then(|v| v.as_str()).unwrap_or("");
                    let state_path = b.get("snapshot_path").and_then(|v| v.as_str()).unwrap_or("");
                    self.active_snapshots.lock().unwrap().insert(mem_path.to_string(), state_path.to_string());
                    Ok(serde_json::json!({ "status": "snapshot created" }))
                } else {
                    Err(ShadowError::Vmm("Missing snapshot payload".into()))
                }
            }
            ("PUT", "/snapshot/load") => {
                let mut state = self.state.lock().unwrap();
                *state = MockVmState::Running;
                Ok(serde_json::json!({ "status": "snapshot restored" }))
            }
            ("GET", "/describe") => {
                let state = *self.state.lock().unwrap();
                let state_str = match state {
                    MockVmState::Running => "Running",
                    MockVmState::Paused => "Paused",
                    MockVmState::Terminated => "Halted",
                    _ => "Uninitialized",
                };
                Ok(serde_json::json!({ "state": state_str, "vcpus": *self.vcpu_count.lock().unwrap(), "ram_mib": *self.memory_size_mib.lock().unwrap() }))
            }
            (m, e) => Err(ShadowError::Vmm(format!("Unhandled mock endpoint: {} {}", m, e))),
        }
    }

    pub fn get_state(&self) -> MockVmState {
        *self.state.lock().unwrap()
    }

    pub fn call_count_for(&self, endpoint: &str) -> usize {
        self.recorded_calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.endpoint == endpoint)
            .count()
    }
}
