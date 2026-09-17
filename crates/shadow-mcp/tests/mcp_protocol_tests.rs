use serde_json::json;
use shadow_core::Result;
use shadow_cow::OverlayManager;
use shadow_mcp::{
    handler::McpHandler,
    protocol::{METHOD_NOT_FOUND, PARSE_ERROR},
    server::McpServer,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_jsonrpc_message_parsing_and_error_handling() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    // 1. Malformed JSON
    let resp = server.process_line("{invalid json").expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["error"]["code"], PARSE_ERROR);

    // 2. Unknown Method
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown/method",
        "params": {}
    });
    let resp = server
        .process_line(&req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["error"]["code"], METHOD_NOT_FOUND);

    // 3. Ping
    let req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "ping"
    });
    let resp = server
        .process_line(&req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["id"], 2);
    assert_eq!(val["result"], json!({}));

    Ok(())
}

#[test]
fn test_mcp_initialize_and_tools_list() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    // 1. Initialize
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "Google Antigravity IDE",
                "version": "2.4.5"
            }
        }
    });
    let resp = server
        .process_line(&init_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["id"], 100);
    assert_eq!(val["result"]["serverInfo"]["name"], "shadow-mcp");
    assert_eq!(val["result"]["protocolVersion"], "2024-11-05");

    // 2. Notification: initialized (no response expected)
    let notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let resp = server.process_line(&notif.to_string());
    assert!(resp.is_none());

    // 3. tools/list
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 101,
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

    assert!(tool_names.contains(&"run_sandboxed_cmd"));
    assert!(tool_names.contains(&"inspect_diff"));
    assert!(tool_names.contains(&"rollback_state"));
    assert!(tool_names.contains(&"promote_change"));
    assert_eq!(tools.len(), 4);

    Ok(())
}

#[test]
fn test_mcp_run_sandboxed_cmd_execution() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_repo = temp_root.path().join("mcp_test_repo");
    fs::create_dir_all(&host_repo)?;
    fs::write(host_repo.join("test.txt"), "hello pristine host\n")?;

    let initial_hash = OverlayManager::compute_directory_sha256(&host_repo)?;

    let handler = McpHandler::new(host_repo.clone())?;
    let server = McpServer::new(handler);

    // Call run_sandboxed_cmd tool
    let call_req = json!({
        "jsonrpc": "2.0",
        "id": 200,
        "method": "tools/call",
        "params": {
            "name": "run_sandboxed_cmd",
            "arguments": {
                "command": "claude",
                "args": ["-p", "Run unit tests"],
                "auto_approve": true
            }
        }
    });

    let resp = server
        .process_line(&call_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["id"], 200);

    let content_text = val["result"]["content"][0]["text"]
        .as_str()
        .expect("Content text expected");
    let payload: serde_json::Value = serde_json::from_str(content_text)?;

    assert_eq!(payload["exit_code"], 0);
    assert_eq!(payload["host_pollution"], false);
    assert!(payload["command"]
        .as_str()
        .unwrap()
        .contains("--dangerously-skip-permissions"));

    // Host remains bit-identical
    let post_hash = OverlayManager::compute_directory_sha256(&host_repo)?;
    assert_eq!(initial_hash, post_hash);

    Ok(())
}

#[test]
fn test_mcp_diff_rollback_and_promote_primitives() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_repo = temp_root.path().join("mcp_full_repo");
    fs::create_dir_all(&host_repo)?;

    let main_file = host_repo.join("index.js");
    fs::write(&main_file, "console.log('v1');\n")?;
    let initial_hash = OverlayManager::compute_directory_sha256(&host_repo)?;

    let handler = McpHandler::new(host_repo.clone())?;
    let upper_dir = {
        let ctx = handler.sandbox_context.lock().unwrap();
        ctx.upper_dir.clone()
    };
    let server = McpServer::new(handler);

    // 1. Introduce mutation in upperdir
    fs::write(upper_dir.join("index.js"), "console.log('v2-upgraded');\n")?;

    // 2. Call inspect_diff
    let diff_req = json!({
        "jsonrpc": "2.0",
        "id": 301,
        "method": "tools/call",
        "params": {
            "name": "inspect_diff",
            "arguments": {}
        }
    });
    let resp = server
        .process_line(&diff_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let diff_payload: serde_json::Value =
        serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;

    assert_eq!(diff_payload["file_count"], 1);
    let diff_text = diff_payload["unified_diff"].as_str().unwrap();
    assert!(diff_text.contains("-console.log('v1');"));
    assert!(diff_text.contains("+console.log('v2-upgraded');"));

    // 3. Call rollback_state (<100ms)
    let rollback_req = json!({
        "jsonrpc": "2.0",
        "id": 302,
        "method": "tools/call",
        "params": {
            "name": "rollback_state",
            "arguments": { "checkpoint_id": "baseline" }
        }
    });
    let resp = server
        .process_line(&rollback_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let roll_payload: serde_json::Value =
        serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;

    assert_eq!(roll_payload["status"], "rolled_back");
    assert_eq!(roll_payload["target_met"], true);
    assert!(!upper_dir.join("index.js").exists());
    assert_eq!(
        initial_hash,
        OverlayManager::compute_directory_sha256(&host_repo)?
    );

    // 4. Simulate verified modification and call promote_change
    fs::write(upper_dir.join("index.js"), "console.log('v3-promoted');\n")?;

    let promote_req = json!({
        "jsonrpc": "2.0",
        "id": 303,
        "method": "tools/call",
        "params": {
            "name": "promote_change",
            "arguments": { "file": "index.js" }
        }
    });
    let resp = server
        .process_line(&promote_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let prom_payload: serde_json::Value =
        serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;

    assert_eq!(prom_payload["promoted_count"], 1);
    let promoted_content = fs::read_to_string(&main_file)?;
    assert_eq!(promoted_content, "console.log('v3-promoted');\n");

    let new_hash = OverlayManager::compute_directory_sha256(&host_repo)?;
    assert_ne!(initial_hash, new_hash);

    Ok(())
}
