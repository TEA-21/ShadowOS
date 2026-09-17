use anyhow::Result;
use shadow_mcp::{McpHandler, McpServer};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Direct tracing logs to stderr so stdout remains strictly reserved for JSON-RPC 2.0 messages
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let workspace_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    eprintln!(
        "[ShadowOS MCP] Initializing server for workspace: {}",
        workspace_path.display()
    );

    let handler = McpHandler::new(workspace_path)?;
    let server = McpServer::new(handler);

    server.run_stdio().await?;
    Ok(())
}
