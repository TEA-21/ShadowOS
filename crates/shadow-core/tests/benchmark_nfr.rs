use bytes::BytesMut;
use shadow_core::{MessageType, ShadowFrame, StreamId};
use std::time::Instant;

#[test]
fn test_nfr_framing_latency_and_throughput() {
    let payload = vec![0xAA; 1024]; // 1KB chunk
    let frame = ShadowFrame::new(MessageType::Stdout, StreamId::Stdout, payload);

    let iterations = 10_000;
    let start = Instant::now();

    let mut stream_buf = BytesMut::with_capacity(1024 * 16);
    let mut total_bytes_transferred = 0;

    for _ in 0..iterations {
        let encoded = frame.encode();
        total_bytes_transferred += encoded.len();
        stream_buf.extend_from_slice(&encoded);

        while let Ok(Some(decoded)) = ShadowFrame::decode(&mut stream_buf) {
            assert_eq!(decoded.msg_type, MessageType::Stdout);
        }
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_millis();
    let us_per_op = elapsed.as_micros() as f64 / iterations as f64;

    println!(
        "\n[NFR BENCHMARK] Processed {} frames ({} MB) in {}ms ({:.2} µs/frame)",
        iterations,
        total_bytes_transferred / (1024 * 1024),
        elapsed_ms,
        us_per_op
    );

    // Assert framing overhead is well under 100 microseconds per frame
    assert!(
        us_per_op < 100.0,
        "AF_VSOCK framing overhead too high: {:.2} µs/frame",
        us_per_op
    );
}

#[test]
fn test_nfr_struct_memory_overhead() {
    // Ensure memory structures are lean to respect the <150MB base RAM limit
    let frame_header_overhead = std::mem::size_of::<ShadowFrame>();
    println!("\n[NFR MEMORY] ShadowFrame struct size: {} bytes", frame_header_overhead);
    assert!(
        frame_header_overhead <= 48,
        "ShadowFrame overhead exceeds 48 bytes: {}",
        frame_header_overhead
    );
}
