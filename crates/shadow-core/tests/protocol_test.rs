use bytes::BytesMut;
use shadow_core::{
    CommandRequest, ExitNotification, MessageType, Result, ShadowError, ShadowFrame, StreamId,
};
use std::collections::HashMap;

#[test]
fn test_frame_encode_decode_roundtrip() -> Result<()> {
    let payload = b"echo 'Hello from ShadowOS MicroVM'".to_vec();
    let original_frame = ShadowFrame::new(MessageType::Stdout, StreamId::Stdout, payload.clone());

    let encoded_bytes = original_frame.encode();
    assert_eq!(&encoded_bytes[0..2], &[0x53, 0x4F]); // Magic 'S', 'O'
    assert_eq!(encoded_bytes[2], 0x02); // Stdout
    assert_eq!(encoded_bytes[3], 0x02); // StreamId::Stdout
    assert_eq!(
        u32::from_be_bytes(encoded_bytes[4..8].try_into().unwrap()),
        payload.len() as u32
    );

    let mut buf = BytesMut::from(&encoded_bytes[..]);
    let decoded = ShadowFrame::decode(&mut buf)?;

    assert!(decoded.is_some());
    let decoded_frame = decoded.unwrap();
    assert_eq!(decoded_frame.msg_type, MessageType::Stdout);
    assert_eq!(decoded_frame.stream_id, StreamId::Stdout);
    assert_eq!(decoded_frame.payload, payload);
    assert_eq!(buf.len(), 0); // All bytes consumed

    Ok(())
}

#[test]
fn test_heartbeat_frame() -> Result<()> {
    let hb = ShadowFrame::heartbeat();
    assert_eq!(hb.msg_type, MessageType::Heartbeat);
    assert_eq!(hb.stream_id, StreamId::Control);
    assert!(hb.payload.is_empty());

    let encoded = hb.encode();
    let mut buf = BytesMut::from(&encoded[..]);
    let decoded = ShadowFrame::decode(&mut buf)?.expect("Heartbeat should decode cleanly");

    assert_eq!(decoded.msg_type, MessageType::Heartbeat);
    assert!(decoded.payload.is_empty());
    Ok(())
}

#[test]
fn test_command_request_and_exit_payloads() -> Result<()> {
    let mut env = HashMap::new();
    env.insert("SANDBOX".to_string(), "1".to_string());
    env.insert("MOCK_KEY".to_string(), "sk-test-mock".to_string());

    let cmd_req = CommandRequest {
        cmd: "/bin/sh".to_string(),
        args: vec!["-c".to_string(), "cargo test".to_string()],
        env,
        workdir: "/workspace".to_string(),
        timeout_seconds: 60,
    };

    let serialized = serde_json::to_vec(&cmd_req)?;
    let frame = ShadowFrame::new(MessageType::CmdReq, StreamId::Control, serialized);

    let encoded = frame.encode();
    let mut buf = BytesMut::from(&encoded[..]);
    let decoded_frame = ShadowFrame::decode(&mut buf)?.expect("Should decode cmd frame");

    let deserialized_cmd: CommandRequest = serde_json::from_slice(&decoded_frame.payload)?;
    assert_eq!(deserialized_cmd.cmd, "/bin/sh");
    assert_eq!(deserialized_cmd.args, vec!["-c", "cargo test"]);
    assert_eq!(deserialized_cmd.env.get("SANDBOX").unwrap(), "1");
    assert_eq!(deserialized_cmd.workdir, "/workspace");

    // Exit notification payload
    let exit_info = ExitNotification {
        exit_code: 0,
        elapsed_ms: 84,
    };
    let exit_payload = serde_json::to_vec(&exit_info)?;
    let exit_frame = ShadowFrame::new(MessageType::Exit, StreamId::Control, exit_payload);
    let mut exit_buf = BytesMut::from(&exit_frame.encode()[..]);
    let decoded_exit = ShadowFrame::decode(&mut exit_buf)?.expect("Should decode exit frame");
    let deserialized_exit: ExitNotification = serde_json::from_slice(&decoded_exit.payload)?;
    assert_eq!(deserialized_exit.exit_code, 0);
    assert_eq!(deserialized_exit.elapsed_ms, 84);

    Ok(())
}

#[test]
fn test_invalid_magic_bytes_rejection() {
    let mut corrupted = BytesMut::from(&[0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x04, b't', b'e', b's', b't'][..]);
    let result = ShadowFrame::decode(&mut corrupted);
    assert!(result.is_err());
    match result.unwrap_err() {
        ShadowError::Protocol(msg) => assert!(msg.contains("Invalid protocol magic bytes")),
        other => panic!("Expected Protocol error variant, got {:?}", other),
    }
}

#[test]
fn test_incomplete_and_streaming_chunk_handling() -> Result<()> {
    let payload = b"large chunk of streamed stdout data from agent".to_vec();
    let frame = ShadowFrame::stdout(payload.clone());
    let encoded = frame.encode();

    // 1. Incomplete header (< 8 bytes)
    let mut partial_buf = BytesMut::from(&encoded[0..5]);
    let res = ShadowFrame::decode(&mut partial_buf)?;
    assert!(res.is_none(), "Should return None when header is truncated");
    assert_eq!(partial_buf.len(), 5, "Buffer must not be consumed");

    // 2. Full header but partial payload
    let mut partial_payload_buf = BytesMut::from(&encoded[0..12]);
    let res = ShadowFrame::decode(&mut partial_payload_buf)?;
    assert!(res.is_none(), "Should return None when payload is incomplete");

    // 3. Complete chunk when remaining bytes arrive
    partial_payload_buf.extend_from_slice(&encoded[12..]);
    let res = ShadowFrame::decode(&mut partial_payload_buf)?;
    assert!(res.is_some());
    assert_eq!(res.unwrap().payload, payload);
    assert_eq!(partial_payload_buf.len(), 0);

    Ok(())
}

#[test]
fn test_consecutive_multi_frame_stream() -> Result<()> {
    let frame1 = ShadowFrame::stdout(b"Frame 1 content".to_vec());
    let frame2 = ShadowFrame::stderr(b"Frame 2 error content".to_vec());
    let frame3 = ShadowFrame::heartbeat();

    let mut stream_buf = BytesMut::new();
    stream_buf.extend_from_slice(&frame1.encode());
    stream_buf.extend_from_slice(&frame2.encode());
    stream_buf.extend_from_slice(&frame3.encode());

    let d1 = ShadowFrame::decode(&mut stream_buf)?.expect("Frame 1 decoded");
    assert_eq!(d1.msg_type, MessageType::Stdout);
    assert_eq!(d1.payload, b"Frame 1 content");

    let d2 = ShadowFrame::decode(&mut stream_buf)?.expect("Frame 2 decoded");
    assert_eq!(d2.msg_type, MessageType::Stderr);
    assert_eq!(d2.payload, b"Frame 2 error content");

    let d3 = ShadowFrame::decode(&mut stream_buf)?.expect("Frame 3 decoded");
    assert_eq!(d3.msg_type, MessageType::Heartbeat);

    assert_eq!(stream_buf.len(), 0);
    assert!(ShadowFrame::decode(&mut stream_buf)?.is_none());

    Ok(())
}
