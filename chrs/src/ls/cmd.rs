use super::coder_channel::{loop_decoder, DecodeChannel};
use super::plain::ls_plain;
use crate::client::{Credentials, RoClient};
use crate::files::decoder::MaybeChrisPathHumanCoder;
use crate::ls::options::WhatToPrint;
use camino::{Utf8Path, Utf8PathBuf};
use chris::types::PluginInstanceId;
use clap::Parser;
use color_eyre::eyre::{bail, Result};
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
    let (client, old_id, _) = credentials.get_client(path.as_slice()).await?;
    let ro_client = client.into_ro();
    let level = level.unwrap_or(if tree { 4 } else { 1 });
    let path = if let Some(p) = path {
        resolve_given_path(&ro_client, old_id.clone(), p).await?
    } else if let Some(id) = old_id {
        pwd(&ro_client, id).await?
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

async fn resolve_given_path(
    client: &RoClient,
    pid: Option<PluginInstanceId>,
    given_path: String,
) -> Result<String> {
    if &given_path == "."
        || ["./", "..", "../"]
            .iter()
            .any(|s| given_path.starts_with(s))
    {
        if let Some(id) = pid {
            let wd = pwd(client, id).await?;
            Ok(reconcile_path(&wd, &given_path))
        } else {
            bail!(
                "Cannot cd into {}: no current plugin instance context",
                given_path
            )
        }
    } else {
        Ok(given_path)
    }
}

async fn pwd(client: &RoClient, id: PluginInstanceId) -> Result<String> {
    let output_path = client.get_plugin_instance(id).await?.object.output_path;
    let wd = output_path
        .strip_suffix("/data")
        .unwrap_or(&output_path)
        .to_string();
    Ok(wd)
}

fn reconcile_path(wd: &str, rel_path: &str) -> String {
    let path = Utf8Path::new(wd).to_path_buf();
    rel_path.split('/').fold(path, reduce_path).to_string()
}

fn reduce_path(acc: Utf8PathBuf, component: &str) -> Utf8PathBuf {
    if component == "." || component.is_empty() {
        acc
    } else if component == ".." {
        acc.parent().map(|p| p.to_path_buf()).unwrap_or(acc)
    } else {
        acc.join(component)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("a/b/c", ".", "a/b/c")]
    #[case("a/b/c", "./d", "a/b/c/d")]
    #[case("a/b/c", "..", "a/b")]
    #[case("a/b/c", "../", "a/b")]
    #[case("a/b/c", "../..", "a")]
    #[case("a/b/c", "..//..", "a")]
    #[case("a/b/c", "..//..//.", "a")]
    fn test_reconcile_path(#[case] wd: &str, #[case] rel_path: &str, #[case] expected: &str) {
        let actual = reconcile_path(wd, rel_path);
        assert_eq!(&actual, expected)
    }
}
