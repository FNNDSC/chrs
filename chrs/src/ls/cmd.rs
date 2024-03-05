use super::coder_channel::{loop_decoder, DecodeChannel};
use super::plain::ls_plain;
use clap::Parser;
// use super::tree::ls_tree;
use crate::files::decoder::MaybeChrisPathHumanCoder;
use crate::get_client::Credentials;
use crate::ls::options::WhatToPrint;
use color_eyre::eyre::Result;
use tokio::join;
use tokio::sync::mpsc::unbounded_channel;

#[derive(Parser)]
pub struct LsArgs {
    /// tree-like output
    #[clap(short, long)]
    pub tree: bool,

    /// Maximum subdirectory depth
    #[clap(short = 'L', long)]
    pub level: Option<u16>,

    /// Show full paths, which may be convenient for copy-paste
    #[clap(short, long)]
    pub full: bool,

    /// Rename folders with feed names and plugin instance titles
    #[clap(short, long)]
    pub rename: bool,

    /// What to print
    #[clap(short, long, default_value_t, value_enum)]
    pub show: WhatToPrint,

    /// directory path
    #[clap()]
    pub path: Option<String>,
}

pub async fn ls(
    credentials: Credentials,
    LsArgs {
        tree,
        level,
        full,
        rename,
        show,
        path,
    }: LsArgs,
) -> Result<()> {
    let (client, pid, _) = credentials.get_client(path.as_slice()).await?;
    let ro_client = client.into_ro();

    let level = level.unwrap_or(if tree { 4 } else { 1 });

    let path = if let Some(p) = path {
        p
    } else if let Some(id) = pid {
        ro_client.get_plugin_instance(id).await?.object.output_path
    } else {
        "".to_string()
    };

    let coder = MaybeChrisPathHumanCoder::new(&ro_client, rename);
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
            ls_plain(&ro_client, &path, level, full, decode_channel, show),
            decoder_loop
        )
    };
    result
}
