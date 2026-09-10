use bytes::BytesMut;
use shadow_core::{
    error::{Result, ShadowError},
    protocol::{CommandRequest, ExitNotification, MessageType, ShadowFrame, StreamId},
};
use std::time::Duration;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub elapsed_ms: u64,
}

pub struct HostVsockMultiplexer {
    frame_tx: Sender<ShadowFrame>,
}

impl HostVsockMultiplexer {
    pub fn new(frame_tx: Sender<ShadowFrame>) -> Self {
        Self { frame_tx }
    }

    /// Sends a command request to the guest agent and streams back stdout and stderr
    pub async fn execute(
        &self,
        cmd_req: CommandRequest,
        incoming_frames: Receiver<ShadowFrame>,
        stdout_stream: Option<Sender<String>>,
    ) -> Result<CommandOutput> {
        self.execute_with_stderr(cmd_req, incoming_frames, stdout_stream, None).await
    }

    /// Sends a command request with support for dual live stdout & stderr channels and timeout enforcement
    pub async fn execute_with_stderr(
        &self,
        cmd_req: CommandRequest,
        mut incoming_frames: Receiver<ShadowFrame>,
        stdout_stream: Option<Sender<String>>,
        stderr_stream: Option<Sender<String>>,
    ) -> Result<CommandOutput> {
        let timeout_duration = if cmd_req.timeout_seconds > 0 {
            Duration::from_secs(cmd_req.timeout_seconds as u64)
        } else {
            Duration::from_secs(300) // Default 5 minute timeout
        };

        let payload = serde_json::to_vec(&cmd_req)?;
        let req_frame = ShadowFrame::new(MessageType::CmdReq, StreamId::Control, payload);

        self.frame_tx.send(req_frame).await.map_err(|_| {
            ShadowError::Vsock("Failed to dispatch command request frame to vsock bridge".into())
        })?;

        let execution_future = async {
            let mut stdout_acc = String::new();
            let mut stderr_acc = String::new();
            let mut final_exit_code = -1;
            let mut final_elapsed_ms = 0u64;

            while let Some(frame) = incoming_frames.recv().await {
                match frame.msg_type {
                    MessageType::Stdout => {
                        let text = String::from_utf8_lossy(&frame.payload).to_string();
                        if let Some(ref tx) = stdout_stream {
                            let _ = tx.send(text.clone()).await;
                        }
                        stdout_acc.push_str(&text);
                    }
                    MessageType::Stderr => {
                        let text = String::from_utf8_lossy(&frame.payload).to_string();
                        if let Some(ref tx) = stderr_stream {
                            let _ = tx.send(text.clone()).await;
                        }
                        stderr_acc.push_str(&text);
                    }
                    MessageType::Exit => {
                        if let Ok(exit_info) = serde_json::from_slice::<ExitNotification>(&frame.payload) {
                            final_exit_code = exit_info.exit_code;
                            final_elapsed_ms = exit_info.elapsed_ms;
                        }
                        break;
                    }
                    MessageType::Heartbeat => {
                        tracing::trace!("Received guest heartbeat over vsock.");
                    }
                    _ => {}
                }
            }

            Ok(CommandOutput {
                stdout: stdout_acc,
                stderr: stderr_acc,
                exit_code: final_exit_code,
                elapsed_ms: final_elapsed_ms,
            })
        };

        match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(res) => res,
            Err(_) => Err(ShadowError::Timeout(timeout_duration.as_millis() as u64)),
        }
    }

    /// Dispatches an out-of-band control signal (e.g. SIGINT = 2, SIGTERM = 15) to cancel guest execution
    pub async fn send_signal(&self, signal: i32) -> Result<()> {
        let signal_payload = signal.to_be_bytes().to_vec();
        let frame = ShadowFrame::new(MessageType::Signal, StreamId::Control, signal_payload);
        self.frame_tx.send(frame).await.map_err(|_| {
            ShadowError::Vsock("Failed to dispatch signal frame over vsock".into())
        })?;
        Ok(())
    }
}
