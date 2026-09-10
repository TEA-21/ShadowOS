use std::collections::HashMap;
use tokio::sync::mpsc::channel;
use shadow_core::{
    protocol::{CommandRequest, ExitNotification, MessageType, ShadowFrame, StreamId},
    Result, ShadowError,
};
use shadow_vsock::HostVsockMultiplexer;

#[tokio::test]
async fn test_stdout_streaming_and_exit_code_0() -> Result<()> {
    let (outbound_tx, mut outbound_rx) = channel::<ShadowFrame>(32);
    let (inbound_tx, inbound_rx) = channel::<ShadowFrame>(32);

    let multiplexer = HostVsockMultiplexer::new(outbound_tx);

    let cmd = CommandRequest {
        cmd: "cargo".to_string(),
        args: vec!["--version".to_string()],
        env: HashMap::new(),
        workdir: ".".to_string(),
        timeout_seconds: 10,
    };

    // Simulated guest runner responding over vsock
    tokio::spawn(async move {
        let req_frame = outbound_rx.recv().await.expect("Expected CmdReq");
        assert_eq!(req_frame.msg_type, MessageType::CmdReq);

        inbound_tx
            .send(ShadowFrame::stdout(b"cargo 1.80.0 (376290b 2024-07-16)\n".to_vec()))
            .await
            .unwrap();

        let exit_bytes = serde_json::to_vec(&ExitNotification {
            exit_code: 0,
            elapsed_ms: 18,
        })
        .unwrap();

        inbound_tx
            .send(ShadowFrame::new(MessageType::Exit, StreamId::Control, exit_bytes))
            .await
            .unwrap();
    });

    let (live_tx, mut live_rx) = channel::<String>(16);
    let output = multiplexer.execute(cmd, inbound_rx, Some(live_tx)).await?;

    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("cargo 1.80.0"));
    assert_eq!(output.stderr, "");

    let live_chunk = live_rx.recv().await.expect("Live chunk expected");
    assert!(live_chunk.contains("cargo 1.80.0"));

    Ok(())
}

#[tokio::test]
async fn test_stderr_streaming_isolation() -> Result<()> {
    let (outbound_tx, mut outbound_rx) = channel::<ShadowFrame>(32);
    let (inbound_tx, inbound_rx) = channel::<ShadowFrame>(32);

    let multiplexer = HostVsockMultiplexer::new(outbound_tx);

    let cmd = CommandRequest {
        cmd: "npm".to_string(),
        args: vec!["test".to_string()],
        env: HashMap::new(),
        workdir: ".".to_string(),
        timeout_seconds: 10,
    };

    tokio::spawn(async move {
        let _ = outbound_rx.recv().await;

        // Emit stderr chunk
        inbound_tx
            .send(ShadowFrame::stderr(b"npm ERR! code ENOENT\n".to_vec()))
            .await
            .unwrap();

        // Emit exit code 1
        let exit_bytes = serde_json::to_vec(&ExitNotification {
            exit_code: 1,
            elapsed_ms: 45,
        })
        .unwrap();

        inbound_tx
            .send(ShadowFrame::new(MessageType::Exit, StreamId::Control, exit_bytes))
            .await
            .unwrap();
    });

    let (live_err_tx, mut live_err_rx) = channel::<String>(16);
    let output = multiplexer
        .execute_with_stderr(cmd, inbound_rx, None, Some(live_err_tx))
        .await?;

    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stdout, "");
    assert!(output.stderr.contains("npm ERR! code ENOENT"));

    let err_chunk = live_err_rx.recv().await.expect("Stderr chunk expected");
    assert!(err_chunk.contains("npm ERR!"));

    Ok(())
}

#[tokio::test]
async fn test_non_zero_exit_code_propagation() -> Result<()> {
    let test_codes = vec![1, 2, 42, 127, 137]; // 137 = SIGKILL OOM

    for target_code in test_codes {
        let (outbound_tx, mut outbound_rx) = channel::<ShadowFrame>(16);
        let (inbound_tx, inbound_rx) = channel::<ShadowFrame>(16);

        let multiplexer = HostVsockMultiplexer::new(outbound_tx);

        let cmd = CommandRequest {
            cmd: "exit".to_string(),
            args: vec![target_code.to_string()],
            env: HashMap::new(),
            workdir: ".".to_string(),
            timeout_seconds: 5,
        };

        tokio::spawn(async move {
            let _ = outbound_rx.recv().await;
            let exit_bytes = serde_json::to_vec(&ExitNotification {
                exit_code: target_code,
                elapsed_ms: 5,
            })
            .unwrap();
            inbound_tx
                .send(ShadowFrame::new(MessageType::Exit, StreamId::Control, exit_bytes))
                .await
                .unwrap();
        });

        let output = multiplexer.execute(cmd, inbound_rx, None).await?;
        assert_eq!(
            output.exit_code, target_code,
            "Exit code must propagate accurately: expected {}, got {}",
            target_code, output.exit_code
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_command_timeout_enforcement() {
    let (outbound_tx, mut outbound_rx) = channel::<ShadowFrame>(16);
    let (_inbound_tx, inbound_rx) = channel::<ShadowFrame>(16);

    let multiplexer = HostVsockMultiplexer::new(outbound_tx);

    let cmd = CommandRequest {
        cmd: "sleep".to_string(),
        args: vec!["10".to_string()],
        env: HashMap::new(),
        workdir: ".".to_string(),
        timeout_seconds: 1, // 1 second timeout
    };

    tokio::spawn(async move {
        // Guest never sends Exit frame, simulating a hang
        let _ = outbound_rx.recv().await;
    });

    let res = multiplexer.execute(cmd, inbound_rx, None).await;
    assert!(res.is_err());
    match res.unwrap_err() {
        ShadowError::Timeout(ms) => assert_eq!(ms, 1000),
        other => panic!("Expected Timeout error, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiline_large_output_streaming() -> Result<()> {
    let (outbound_tx, mut outbound_rx) = channel::<ShadowFrame>(1024);
    let (inbound_tx, inbound_rx) = channel::<ShadowFrame>(1024);

    let multiplexer = HostVsockMultiplexer::new(outbound_tx);

    let line_count = 500;
    tokio::spawn(async move {
        let _ = outbound_rx.recv().await;
        for i in 1..=line_count {
            let line = format!("Log line {:04}: Agent test execution underway\n", i);
            inbound_tx
                .send(ShadowFrame::stdout(line.into_bytes()))
                .await
                .unwrap();
        }
        let exit_bytes = serde_json::to_vec(&ExitNotification {
            exit_code: 0,
            elapsed_ms: 120,
        })
        .unwrap();
        inbound_tx
            .send(ShadowFrame::new(MessageType::Exit, StreamId::Control, exit_bytes))
            .await
            .unwrap();
    });

    let cmd = CommandRequest {
        cmd: "generate_logs".to_string(),
        args: vec![],
        env: HashMap::new(),
        workdir: ".".to_string(),
        timeout_seconds: 10,
    };

    let output = multiplexer.execute(cmd, inbound_rx, None).await?;
    assert_eq!(output.exit_code, 0);

    let received_lines = output.stdout.lines().count();
    assert_eq!(
        received_lines, line_count,
        "All streamed lines must be received without drops: expected {}, got {}",
        line_count, received_lines
    );

    Ok(())
}
