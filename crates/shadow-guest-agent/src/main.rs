mod exec;
mod vsock_server;

use tokio::sync::mpsc::channel;
use shadow_core::protocol::ShadowFrame;
use vsock_server::GuestVsockServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("ShadowOS Guest Agent v0.1.0 starting...");

    let port: u32 = std::env::var("SHADOW_VSOCK_PORT")
        .unwrap_or_else(|_| "5001".to_string())
        .parse()
        .unwrap_or(5001);

    let server = GuestVsockServer::new(port);
    let (tx, mut rx) = channel::<ShadowFrame>(128);

    tracing::info!("Guest agent listening on AF_VSOCK port {}", port);

    // Main event pump loop
    while let Some(outbound_frame) = rx.recv().await {
        tracing::trace!("Dispatching outbound frame: {:?}", outbound_frame.msg_type);
    }

    Ok(())
}
