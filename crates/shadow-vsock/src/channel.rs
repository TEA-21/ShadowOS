use tokio::sync::mpsc::{channel, Receiver, Sender};
use shadow_core::protocol::ShadowFrame;

pub struct VsockChannel {
    pub tx: Sender<ShadowFrame>,
    pub rx: Receiver<ShadowFrame>,
}

impl VsockChannel {
    pub fn new(capacity: usize) -> (Self, Sender<ShadowFrame>, Receiver<ShadowFrame>) {
        let (outbound_tx, outbound_rx) = channel(capacity);
        let (inbound_tx, inbound_rx) = channel(capacity);

        let endpoint = Self {
            tx: outbound_tx,
            rx: inbound_rx,
        };

        (endpoint, inbound_tx, outbound_rx)
    }
}
