use serde_json::json;
use shadow_core::Result;
use shadow_mcp::{handler::McpHandler, server::McpServer};
use tempfile::TempDir;

#[test]
fn test_mcp_tool_catalog_includes_visual_browser_primitives() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 500,
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

    assert_eq!(tools.len(), 8);
    Ok(())
}

#[test]
fn test_mcp_browser_navigate_and_screenshot() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    // 1. browser_navigate
    let nav_req = json!({
        "jsonrpc": "2.0",
        "id": 501,
        "method": "tools/call",
        "params": {
            "name": "browser_navigate",
            "arguments": {
                "url": "http://127.0.0.1:8080/dashboard"
            }
        }
    });

    let resp = server
        .process_line(&nav_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["id"], 501);

    let content_text = val["result"]["content"][0]["text"].as_str().unwrap();
    let nav_payload: serde_json::Value = serde_json::from_str(content_text)?;

    assert_eq!(nav_payload["url"], "http://127.0.0.1:8080/dashboard");
    assert_eq!(nav_payload["status"], 200);
    assert_eq!(nav_payload["ready_state"], "complete");
    assert_eq!(nav_payload["display"], ":99");
    assert_eq!(nav_payload["host_screen_pollution"], false);

    // 2. capture_screenshot
    let shot_req = json!({
        "jsonrpc": "2.0",
        "id": 502,
        "method": "tools/call",
        "params": {
            "name": "capture_screenshot",
            "arguments": {
                "format": "png",
                "full_page": false
            }
        }
    });

    let resp = server
        .process_line(&shot_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    assert_eq!(val["id"], 502);

    let content_text = val["result"]["content"][0]["text"].as_str().unwrap();
    let shot_payload: serde_json::Value = serde_json::from_str(content_text)?;

    assert_eq!(shot_payload["format"], "png");
    assert_eq!(shot_payload["width"], 1920);
    assert_eq!(shot_payload["height"], 1080);
    assert!(shot_payload["byte_size"].as_u64().unwrap() > 0);
    assert!(shot_payload["render_latency_ms"].as_f64().unwrap() < 100.0);
    assert_eq!(shot_payload["target_met"], true);
    assert_eq!(shot_payload["host_screen_pollution"], false);

    Ok(())
}

#[test]
fn test_mcp_browser_click_and_type_interactions() -> Result<()> {
    let temp_root = TempDir::new()?;
    let handler = McpHandler::new(temp_root.path().to_path_buf())?;
    let server = McpServer::new(handler);

    // 1. browser_click
    let click_req = json!({
        "jsonrpc": "2.0",
        "id": 503,
        "method": "tools/call",
        "params": {
            "name": "browser_click",
            "arguments": {
                "x": 250.0,
                "y": 400.0,
                "selector": "#submit-btn"
            }
        }
    });

    let resp = server
        .process_line(&click_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let click_payload: serde_json::Value =
        serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;

    assert_eq!(click_payload["status"], "clicked");
    assert_eq!(click_payload["coordinates"][0], 250.0);
    assert_eq!(click_payload["coordinates"][1], 400.0);
    assert_eq!(click_payload["host_screen_pollution"], false);

    // 2. browser_type
    let type_req = json!({
        "jsonrpc": "2.0",
        "id": 504,
        "method": "tools/call",
        "params": {
            "name": "browser_type",
            "arguments": {
                "text": "cargo run --release",
                "selector": "#input-command"
            }
        }
    });

    let resp = server
        .process_line(&type_req.to_string())
        .expect("Response expected");
    let val: serde_json::Value = serde_json::from_str(&resp)?;
    let type_payload: serde_json::Value =
        serde_json::from_str(val["result"]["content"][0]["text"].as_str().unwrap())?;

    assert_eq!(type_payload["status"], "typed");
    assert_eq!(type_payload["characters_sent"], 19);
    assert_eq!(type_payload["host_screen_pollution"], false);

    Ok(())
}
