use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::mpsc::Sender;
use shadow_core::{
    protocol::{CommandRequest, ExitNotification, MessageType, ShadowFrame, StreamId},
    Result, ShadowError,
};

pub struct CommandExecutor;

impl CommandExecutor {
    pub async fn run(
        req: CommandRequest,
        frame_tx: Sender<ShadowFrame>,
    ) -> Result<i32> {
        let start = std::time::Instant::now();
        let mut cmd = Command::new(&req.cmd);
        cmd.args(&req.args);
        
        let workdir = if req.workdir.is_empty() {
            "/workspace"
        } else {
            &req.workdir
        };
        cmd.current_dir(workdir);

        // Populate environment variables
        for (k, v) in req.env {
            cmd.env(k, v);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        tracing::info!("Guest executing command: {} {:?}", req.cmd, req.args);

        let mut child = cmd.spawn().map_err(|e| {
            ShadowError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to spawn child process: {}", e),
            ))
        })?;

        let mut stdout_pipe = child.stdout.take().ok_or_else(|| {
            ShadowError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Failed to capture stdout"))
        })?;

        let mut stderr_pipe = child.stderr.take().ok_or_else(|| {
            ShadowError::Io(std::io::Error::new(std::io::ErrorKind::Other, "Failed to capture stderr"))
        })?;

        let tx_out = frame_tx.clone();
        let stdout_handle = tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stdout_pipe.read(&mut buf).await {
                if n == 0 { break; }
                let frame = ShadowFrame::stdout(buf[..n].to_vec());
                if tx_out.send(frame).await.is_err() { break; }
            }
        });

        let tx_err = frame_tx.clone();
        let stderr_handle = tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stderr_pipe.read(&mut buf).await {
                if n == 0 { break; }
                let frame = ShadowFrame::stderr(buf[..n].to_vec());
                if tx_err.send(frame).await.is_err() { break; }
            }
        });

        let status = child.wait().await?;
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        let exit_code = status.code().unwrap_or(-1);
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let exit_payload = serde_json::to_vec(&ExitNotification {
            exit_code,
            elapsed_ms,
        })?;

        let exit_frame = ShadowFrame::new(MessageType::Exit, StreamId::Control, exit_payload);
        let _ = frame_tx.send(exit_frame).await;

        tracing::info!("Command finished with code {} in {}ms", exit_code, elapsed_ms);
        Ok(exit_code)
    }
}
