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

    pub async fn decode(&mut self, fname: &str) -> String {
        // TODO can I avoid this clone? https://github.com/dcchut/async-recursion/issues/37
        self.tx_fname.send(fname.to_string()).unwrap();
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
