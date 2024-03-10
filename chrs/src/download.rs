use camino::Utf8PathBuf;
use clap::Parser;
use color_eyre::eyre;
use crate::arg::GivenDataNode;
use crate::credentials::Credentials;

#[derive(Parser)]
pub struct DownloadArgs {
    /// Save files from under plugin instances' "data" subdirectory at
    /// the top-level, instead of under the nested parent directory.
    ///
    /// May be repeated to handle cases where the `data` subdirectory
    /// is deeply nested under parent `data` subdirectories, e.g. `-sssss`.
    #[clap(short, long, action = clap::ArgAction::Count)]
    shorten: u8,

    /// Save as canonical folder names instead of renaming them to feed names
    /// or plugin instance titles
    #[clap(short, long)]
    pub no_titles: bool,

    /// Join contents of all "data" folders to the same output directory.
    ///
    /// Useful when trying to download sibling plugin instance outputs.
    #[clap(short, long, hide = true)]
    flatten: bool,

    /// Skip downloading of files which already exist on the filesystem,
    /// and where their file sizes match what is expected.
    #[clap(long)]
    skip_existing: bool,

    /// Overwrite existing files
    #[clap(long, conflicts_with = "skip_existing")]
    clobber: bool,

    /// What to download.
    src: GivenDataNode,

    /// Directory where to download
    dst: Option<Utf8PathBuf>,
}

pub async fn download(credentials: Credentials, args: DownloadArgs) -> eyre::Result<()> {
    let (client, old, _) = credentials.get_client([args.src.as_arg_str()]).await?;
    let src_path = args.src.into_path(&client, old).await?;


    Ok(())
}
