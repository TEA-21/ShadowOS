use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use crate::error::{Result, ShadowError};

pub const MAGIC: [u8; 2] = [0x53, 0x4F]; // 'S', 'O'
pub const HEADER_LEN: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    CmdReq = 0x01,
    Stdout = 0x02,
    Stderr = 0x03,
    Exit = 0x04,
    Heartbeat = 0x05,
    Signal = 0x06,
}

impl TryFrom<u8> for MessageType {
    type Error = ShadowError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(MessageType::CmdReq),
            0x02 => Ok(MessageType::Stdout),
            0x03 => Ok(MessageType::Stderr),
            0x04 => Ok(MessageType::Exit),
            0x05 => Ok(MessageType::Heartbeat),
            0x06 => Ok(MessageType::Signal),
            other => Err(ShadowError::Protocol(format!("Unknown message type: 0x{:02X}", other))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum StreamId {
    Control = 0x00,
    Stdin = 0x01,
    Stdout = 0x02,
    Stderr = 0x03,
}

impl TryFrom<u8> for StreamId {
    type Error = ShadowError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(StreamId::Control),
            0x01 => Ok(StreamId::Stdin),
            0x02 => Ok(StreamId::Stdout),
            0x03 => Ok(StreamId::Stderr),
            other => Err(ShadowError::Protocol(format!("Unknown stream id: 0x{:02X}", other))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub cmd: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub workdir: String,
    pub timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitNotification {
    pub exit_code: i32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowFrame {
    pub msg_type: MessageType,
    pub stream_id: StreamId,
    pub payload: Vec<u8>,
}

impl ShadowFrame {
    pub fn new(msg_type: MessageType, stream_id: StreamId, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            stream_id,
            payload,
        }
    }

    pub fn heartbeat() -> Self {
        Self::new(MessageType::Heartbeat, StreamId::Control, Vec::new())
    }

    pub fn stdout(data: Vec<u8>) -> Self {
        Self::new(MessageType::Stdout, StreamId::Stdout, data)
    }

    pub fn stderr(data: Vec<u8>) -> Self {
        Self::new(MessageType::Stderr, StreamId::Stderr, data)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(HEADER_LEN + self.payload.len());
        buf.put_slice(&MAGIC);
        buf.put_u8(self.msg_type as u8);
        buf.put_u8(self.stream_id as u8);
        buf.put_u32(self.payload.len() as u32);
        buf.put_slice(&self.payload);
        buf.to_vec()
    }

    pub fn decode(src: &mut BytesMut) -> Result<Option<Self>> {
        if src.len() < HEADER_LEN {
            return Ok(None);
        }

        if src[0] != MAGIC[0] || src[1] != MAGIC[1] {
            return Err(ShadowError::Protocol(format!(
                "Invalid protocol magic bytes: [0x{:02X}, 0x{:02X}]",
                src[0], src[1]
            )));
        }

        let msg_type = MessageType::try_from(src[2])?;
        let stream_id = StreamId::try_from(src[3])?;
        let payload_len = (&src[4..8]).get_u32() as usize;

        if src.len() < HEADER_LEN + payload_len {
            return Ok(None);
        }

        src.advance(HEADER_LEN);
        let payload = src.split_to(payload_len).to_vec();

        Ok(Some(Self {
            msg_type,
            stream_id,
            payload,
        }))
    }
}
