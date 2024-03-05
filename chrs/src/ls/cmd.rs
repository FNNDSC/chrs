use super::coder_channel::{loop_decoder, DecodeChannel};
use super::plain::ls_plain;
// use super::tree::ls_tree;
use crate::files::decoder::MaybeChrisPathHumanCoder;
use crate::get_client::{get_client, Credentials};
use color_eyre::eyre::Result;
use tokio::join;
use tokio::sync::mpsc::unbounded_channel;

pub async fn ls(
    credentials: Credentials,
    level: Option<u16>,
    path: Option<String>,
    retries: Option<u32>,
    tree: bool,
    full: bool,
    raw: bool,
) -> Result<()> {
    let (client, pid) = get_client(credentials, path.as_slice(), retries).await?;
    let ro_client = client.into_ro();

    let level = level.unwrap_or(if tree { 4 } else { 1 });

    let path = if let Some(p) = path {
        p
    } else if let Some(id) = pid {
        ro_client.get_plugin_instance(id).await?.object.output_path
    } else {
        "".to_string()
    };

    let coder = MaybeChrisPathHumanCoder::new(&ro_client, !raw);
    let (tx_fname, rx_fname) = unbounded_channel();
    let (tx_decoded, rx_decoded) = unbounded_channel();
    let decode_channel = DecodeChannel::new(tx_fname, rx_decoded);
    let decoder_loop = loop_decoder(coder, rx_fname, tx_decoded);

    let (result, _) = if tree {
        todo!()
        // join!(
        //     ls_tree(&ro_client, &path, level, full, decode_channel),
        //     decoder_loop
        // )
    } else {
        join!(
            ls_plain(&ro_client, &path, level, full, decode_channel),
            decoder_loop
        )
    };
    result
}
