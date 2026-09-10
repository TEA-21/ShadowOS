use tokio::sync::mpsc::{channel, Receiver, Sender};
use bytes::BytesMut;
use shadow_core::{
    error::{Result, ShadowError},
    protocol::{CommandRequest, ExitNotification, MessageType, ShadowFrame, StreamId},
};

pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
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
        mut incoming_frames: Receiver<ShadowFrame>,
        mut stdout_stream: Option<Sender<String>>,
    ) -> Result<CommandOutput> {
        let payload = serde_json::to_vec(&cmd_req)?;
        let req_frame = ShadowFrame::new(MessageType::CmdReq, StreamId::Control, payload);

        self.frame_tx.send(req_frame).await.map_err(|_| {
            ShadowError::Vsock("Failed to dispatch command request frame to vsock bridge".into())
        })?;

        let mut stdout_acc = String::new();
        let mut stderr_acc = String::new();
        let mut final_exit_code = -1;

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
                    stderr_acc.push_str(&text);
                }
                MessageType::Exit => {
                    if let Ok(exit_info) = serde_json::from_slice::<ExitNotification>(&frame.payload) {
                        final_exit_code = exit_info.exit_code;
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
        })
    }
}
