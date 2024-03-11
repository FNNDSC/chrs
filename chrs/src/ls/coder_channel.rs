use std::future::Future;
use crate::files::decoder::MaybeChrisPathHumanCoder;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

/// A channel for communicating with [MaybeChrisPathHumanCoder] in async contexts.
pub struct DecodeChannel {
    tx_fname: UnboundedSender<String>,
    rx_decoded: UnboundedReceiver<String>,
}

impl DecodeChannel {
    pub fn create(coder: MaybeChrisPathHumanCoder) -> (Self, impl Future<Output=()> + '_) {
        let (tx_fname, rx_fname) = unbounded_channel();
        let (tx_decoded, rx_decoded) = unbounded_channel();
        let decoder_channel = Self {
            tx_fname,
            rx_decoded,
        };
        let decoder_loop = loop_decoder(coder, rx_fname, tx_decoded);
        (decoder_channel, decoder_loop)
    }

    /// Calls [MaybeChrisPathHumanCoder::decode]
    pub async fn decode(&mut self, fname: String) -> String {
        self.tx_fname.send(fname).unwrap();
        self.rx_decoded.recv().await.unwrap()
    }
}

#[allow(clippy::needless_lifetimes)]
async fn loop_decoder<'a>(
    mut coder: MaybeChrisPathHumanCoder<'a>,
    mut rx: UnboundedReceiver<String>,
    tx: UnboundedSender<String>,
) {
    while let Some(fname) = rx.recv().await {
        tx.send(coder.decode(&fname).await).unwrap()
    }
}
