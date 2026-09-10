use bytes::BytesMut;
use shadow_core::{
    protocol::{CommandRequest, ExitNotification, MessageType, ShadowFrame, StreamId},
    Result,
};
use shadow_vsock::{HostVsockMultiplexer, VsockChannel};
use std::collections::HashMap;
use tokio::sync::mpsc::channel;

#[tokio::test]
async fn test_vsock_channel_queue_pairing() {
    let (mut endpoint, inbound_tx, mut outbound_rx) = VsockChannel::new(16);

    let frame_to_send = ShadowFrame::stdout(b"test output".to_vec());
    endpoint.tx.send(frame_to_send.clone()).await.unwrap();

    let received_outbound = outbound_rx.recv().await.unwrap();
    assert_eq!(received_outbound, frame_to_send);

    let incoming_frame = ShadowFrame::heartbeat();
    inbound_tx.send(incoming_frame.clone()).await.unwrap();

    let received_inbound = endpoint.rx.recv().await.unwrap();
    assert_eq!(received_inbound, incoming_frame);
}

#[tokio::test]
async fn test_host_multiplexer_execution_flow() -> Result<()> {
    let (outbound_tx, mut outbound_rx) = channel::<ShadowFrame>(16);
    let (inbound_tx, inbound_rx) = channel::<ShadowFrame>(16);

    let multiplexer = HostVsockMultiplexer::new(outbound_tx);

    let cmd_req = CommandRequest {
        cmd: "npm".to_string(),
        args: vec!["test".to_string()],
        env: HashMap::new(),
        workdir: "/workspace".to_string(),
        timeout_seconds: 30,
    };

    // Simulate guest response pipeline in background task
    let guest_simulator = tokio::spawn(async move {
        // 1. Receive command request frame
        let req_frame = outbound_rx.recv().await.expect("Expected command request frame");
        assert_eq!(req_frame.msg_type, MessageType::CmdReq);

        // 2. Stream chunk 1 of stdout
        inbound_tx
            .send(ShadowFrame::stdout(b"RUNS 14 tests...\n".to_vec()))
            .await
            .unwrap();

        // 3. Stream chunk 2 of stdout
        inbound_tx
            .send(ShadowFrame::stdout(b"PASS All 14 tests passed!\n".to_vec()))
            .await
            .unwrap();

        // 4. Send Exit notification
        let exit_payload = serde_json::to_vec(&ExitNotification {
            exit_code: 0,
            elapsed_ms: 112,
        })
        .unwrap();
        inbound_tx
            .send(ShadowFrame::new(
                MessageType::Exit,
                StreamId::Control,
                exit_payload,
            ))
            .await
            .unwrap();
    });

    let (live_stdout_tx, mut live_stdout_rx) = channel::<String>(16);
    let result = multiplexer
        .execute(cmd_req, inbound_rx, Some(live_stdout_tx))
        .await?;

    guest_simulator.await.unwrap();

    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.contains("All 14 tests passed!"));

    // Verify live stream was also received in real-time chunks
    let first_chunk = live_stdout_rx.recv().await.unwrap();
    assert_eq!(first_chunk, "RUNS 14 tests...\n");

    let second_chunk = live_stdout_rx.recv().await.unwrap();
    assert_eq!(second_chunk, "PASS All 14 tests passed!\n");

    Ok(())
}
