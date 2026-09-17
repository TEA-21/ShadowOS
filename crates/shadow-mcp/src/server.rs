use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::handler::McpHandler;
use crate::protocol::{JsonRpcRequest, JsonRpcResponse, PARSE_ERROR};

pub struct McpServer {
    handler: Arc<McpHandler>,
}

impl McpServer {
    pub fn new(handler: McpHandler) -> Self {
        Self {
            handler: Arc::new(handler),
        }
    }

    /// Processes a single raw input line and returns the JSON-RPC 2.0 response string if applicable
    pub fn process_line(&self, line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(
                    None,
                    PARSE_ERROR,
                    format!("Invalid JSON-RPC 2.0 payload: {}", e),
                );
                return serde_json::to_string(&err_resp).ok();
            }
        };

        if let Some(resp) = self.handler.handle_request(request) {
            serde_json::to_string(&resp).ok()
        } else {
            None
        }
    }

    /// Runs the stdio communication loop asynchronously
    pub async fn run_stdio(&self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        tracing::info!("ShadowOS MCP Server listening on stdio...");

        while let Some(line) = lines.next_line().await? {
            if let Some(response_str) = self.process_line(&line) {
                stdout.write_all(response_str.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }

        tracing::info!("MCP Server stdio stream closed.");
        Ok(())
    }
}
