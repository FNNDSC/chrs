use clap::Parser;
use color_eyre::eyre::Result;
use tokio::join;

use crate::arg::GivenPluginInstanceOrPath;
use crate::credentials::Credentials;
use crate::files::{CoderChannel, MaybeChrisPathHumanCoder};
use crate::ls::options::WhatToPrint;

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

    /// Show canonical folder names instead of renaming them to feed names or plugin instance titles
    #[clap(short, long)]
    pub no_titles: bool,

    /// What to print
    #[clap(short, long, default_value_t, value_enum)]
    pub show: WhatToPrint,

    /// directory path or plugin instance
    #[clap(default_value_t)]
    pub path: GivenPluginInstanceOrPath,
}

pub async fn ls(
    credentials: Credentials,
    LsArgs {
        tree,
        level,
        full,
        no_titles,
        show,
        path,
    }: LsArgs,
) -> Result<()> {
    let (client, old_id, _) = credentials.get_client([path.as_arg_str()]).await?;
    let level = level.unwrap_or(if tree { 4 } else { 1 });
    let path = path.into_path(&client, old_id).await?;

    let ro_client = client.into_ro();
    let coder = MaybeChrisPathHumanCoder::new(&ro_client, !no_titles);
    let (decode_channel, decoder_loop) = CoderChannel::create(coder);

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
