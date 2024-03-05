use crate::files::decoder::MaybeChrisPathHumanCoder;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct DecodeChannel {
    tx_fname: UnboundedSender<String>,
    rx_decoded: UnboundedReceiver<String>,
}

impl DecodeChannel {
    pub fn new(tx_fname: UnboundedSender<String>, rx_decoded: UnboundedReceiver<String>) -> Self {
        Self {
            tx_fname,
            rx_decoded,
        }
    }

    pub async fn decode(&mut self, fname: String) -> String {
        self.tx_fname.send(fname).unwrap();
        self.rx_decoded.recv().await.unwrap()
    }
}

#[allow(clippy::needless_lifetimes)]
pub(crate) async fn loop_decoder<'a>(
    mut coder: MaybeChrisPathHumanCoder<'a>,
    mut rx: UnboundedReceiver<String>,
    tx: UnboundedSender<String>,
) {
    while let Some(fname) = rx.recv().await {
        tx.send(coder.decode(&fname).await).unwrap()
    }
}
