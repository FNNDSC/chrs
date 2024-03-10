use crate::arg::GivenPluginInstance;
use clap::Parser;
use color_eyre::eyre::Result;
use tokio::join;
use tokio::sync::mpsc::unbounded_channel;

use crate::credentials::Credentials;
use crate::files::decoder::MaybeChrisPathHumanCoder;
use crate::ls::options::WhatToPrint;

use super::coder_channel::{loop_decoder, DecodeChannel};
use super::plain::ls_plain;

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

    /// Show folders with feed names and plugin instance titles
    #[clap(short = 'n', long)]
    pub show_names: bool,

    /// What to print
    #[clap(short, long, default_value_t, value_enum)]
    pub show: WhatToPrint,

    /// directory path or plugin instance
    #[clap(default_value_t)]
    pub path: GivenPluginInstance,
}

pub async fn ls(
    credentials: Credentials,
    LsArgs {
        tree,
        level,
        full,
        show_names,
        show,
        path,
    }: LsArgs,
) -> Result<()> {
    let (client, old_id, _) = credentials.get_client([path.as_arg_str()]).await?;
    let level = level.unwrap_or(if tree { 4 } else { 1 });
    let path = path.get_as_path(&client, old_id).await?;

    let ro_client = client.into_ro();
    let coder = MaybeChrisPathHumanCoder::new(&ro_client, show_names);
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
