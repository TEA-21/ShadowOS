use serde_json::json;
use shadow_core::Result;
use shadow_mcp::{handler::McpHandler, server::McpServer};
use tempfile::TempDir;

#[test]
fn test_mcp_tool_catalog_includes_all_11_primitives() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 600,
        "method": "tools/list"
    });

    let resp = server
        .process_line(&list_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;

    let tools = val["result"]["tools"]
        .as_array()
        .expect("Tools array expected");

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    // 4 Execution Primitives
    assert!(tool_names.contains(&"run_sandboxed_cmd"));
    assert!(tool_names.contains(&"inspect_diff"));
    assert!(tool_names.contains(&"rollback_state"));
    assert!(tool_names.contains(&"promote_change"));

    // 4 Visual Browser Primitives
    assert!(tool_names.contains(&"browser_navigate"));
    assert!(tool_names.contains(&"browser_click"));
    assert!(tool_names.contains(&"browser_type"));
    assert!(tool_names.contains(&"capture_screenshot"));

    // 3 Swarm Primitives
    assert!(tool_names.contains(&"swarm_spawn_worker"));
    assert!(tool_names.contains(&"swarm_dispatch_task"));
    assert!(tool_names.contains(&"swarm_collect_results"));

    assert_eq!(tools.len(), 11);
    Ok(())
}

#[test]
fn test_mcp_swarm_worker_spawn_dispatch_and_collect() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    // 1. Spawn worker-lint
    let spawn1_req = json!({
        "jsonrpc": "2.0",
        "id": 601,
        "method": "tools/call",
        "params": {
            "name": "swarm_spawn_worker",
            "arguments": {
                "worker_id": "worker-lint",
                "cpu_core": 0,
                "memory_mb": 512
            }
        }
    });
    let resp = server.process_line(&spawn1_req.to_string()).expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let p1: serde_json::Value = serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;
    assert_eq!(p1["worker_id"], "worker-lint");
    assert_eq!(p1["pinned_cpu"], 0);
    assert_eq!(p1["target_met"], true);

    // 2. Spawn worker-test
    let spawn2_req = json!({
        "jsonrpc": "2.0",
        "id": 602,
        "method": "tools/call",
        "params": {
            "name": "swarm_spawn_worker",
            "arguments": {
                "worker_id": "worker-test",
                "cpu_core": 1,
                "memory_mb": 512
            }
        }
    });
    let resp = server.process_line(&spawn2_req.to_string()).expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let p2: serde_json::Value = serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;
    assert_eq!(p2["worker_id"], "worker-test");
    assert_eq!(p2["pinned_cpu"], 1);

    // 3. Dispatch task to worker-lint
    let task1_req = json!({
        "jsonrpc": "2.0",
        "id": 603,
        "method": "tools/call",
        "params": {
            "name": "swarm_dispatch_task",
            "arguments": {
                "worker_id": "worker-lint",
                "command": "cargo clippy --fix",
                "args": ["--allow-no-vcs"]
            }
        }
    });
    let resp = server.process_line(&task1_req.to_string()).expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let t1: serde_json::Value = serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;
    assert_eq!(t1["exit_code"], 0);
    assert_eq!(t1["worker_id"], "worker-lint");

    // 4. Dispatch task to worker-test
    let task2_req = json!({
        "jsonrpc": "2.0",
        "id": 604,
        "method": "tools/call",
        "params": {
            "name": "swarm_dispatch_task",
            "arguments": {
                "worker_id": "worker-test",
                "command": "cargo test --workspace"
            }
        }
    });
    let resp = server.process_line(&task2_req.to_string()).expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let t2: serde_json::Value = serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;
    assert_eq!(t2["exit_code"], 0);
    assert_eq!(t2["worker_id"], "worker-test");

    // 5. Collect swarm results
    let collect_req = json!({
        "jsonrpc": "2.0",
        "id": 605,
        "method": "tools/call",
        "params": {
            "name": "swarm_collect_results",
            "arguments": {}
        }
    });
    let resp = server.process_line(&collect_req.to_string()).expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let summary: serde_json::Value = serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;

    assert_eq!(summary["active_workers"], 2);
    assert_eq!(summary["total_tasks_completed"], 2);
    assert_eq!(summary["within_peak_memory_limit"], true);
    assert_eq!(summary["zero_cross_pollution"], true);

    Ok(())
}
