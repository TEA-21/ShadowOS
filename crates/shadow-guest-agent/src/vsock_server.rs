use bytes::BytesMut;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use shadow_core::{
    protocol::{CommandRequest, MessageType, ShadowFrame},
    Result, ShadowError,
};
use crate::exec::CommandExecutor;

pub struct GuestVsockServer {
    port: u32,
}

impl GuestVsockServer {
    pub fn new(port: u32) -> Self {
        Self { port }
    }

    pub async fn handle_incoming_frame(
        &self,
        frame: ShadowFrame,
        frame_tx: Sender<ShadowFrame>,
    ) -> Result<()> {
        match frame.msg_type {
            MessageType::CmdReq => {
                let cmd_req: CommandRequest = serde_json::from_slice(&frame.payload)?;
                tokio::spawn(async move {
                    if let Err(e) = CommandExecutor::run(cmd_req, frame_tx).await {
                        tracing::error!("Command execution error: {}", e);
                    }
                });
            }
            MessageType::Heartbeat => {
                let hb_ack = ShadowFrame::heartbeat();
                let _ = frame_tx.send(hb_ack).await;
            }
            _ => {
                tracing::warn!("Unhandled frame message type: {:?}", frame.msg_type);
            }
        }
        Ok(())
    }
}
