use camino::Utf8PathBuf;
use clap::Parser;
use color_eyre::eyre;
use color_eyre::eyre::{bail, Context};
use fs_err::tokio::File;
use futures::TryStreamExt;
use futures::{TryFutureExt, TryStream};
use indicatif::ProgressBar;
use tokio_util::io::StreamReader;

use chris::search::Search;
use chris::types::PluginInstanceId;
use chris::{BasicFileResponse, Downloadable, EitherClient, RoAccess};

use crate::arg::{FeedOrPluginInstance, GivenDataNode};
use crate::credentials::Credentials;
use crate::progress::progress_bar_bytes;

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
    let files = get_files_search(&client, args.src.clone(), old).await?;
    download_files(files, args).await
}

type Files = Search<BasicFileResponse, RoAccess>;

async fn get_files_search(
    client: &EitherClient,
    given: GivenDataNode,
    old: Option<PluginInstanceId>,
) -> eyre::Result<Files> {
    match client {
        EitherClient::LoggedIn(logged_in) => {
            let path = given.into_path(&client, old).await?;
            Ok(logged_in.files().fname(path).into_ro().search())
        }
        EitherClient::Anon(_) => {
            let feed_or_plinst = given.into_or(&client, old).await
                .wrap_err_with(|| "Cannot download arbitrary paths unless logged in due to a backend limitation. See https://github.com/FNNDSC/chrs/issues/32")?;
            match feed_or_plinst {
                FeedOrPluginInstance::Feed(f) => Ok(f.files()),
                FeedOrPluginInstance::PluginInstance(p) => Ok(p.files()),
            }
        }
    }
}

async fn download_files(files: Files, args: DownloadArgs) -> eyre::Result<()> {
    let count = files.get_count().await?;
    if count == 0 {
        bail!("No files found")
    };
    if count == 1 {
        download_single_file(files, args).await
    } else {
        download_many_files(files, args).await
    }
}

/// Download one file, showing a progress bar.
async fn download_single_file(files: Files, args: DownloadArgs) -> eyre::Result<()> {
    let only_file = files.get_only().await?;
    let dst = args
        .dst
        .unwrap_or_else(|| Utf8PathBuf::from(only_file.object.basename().to_string()));
    let file = File::create(dst).await?;
    let stream = only_file
        .stream()
        .await?
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e));
    let mut reader = StreamReader::new(stream);
    let pb = progress_bar_bytes(only_file.object.fsize());
    tokio::io::copy(&mut reader, &mut pb.wrap_async_write(file)).await?;
    Ok(())
}

async fn download_many_files(files: Files, args: DownloadArgs) -> eyre::Result<()> {
    todo!()
}

//
// async fn download_files<S: TryStream<Ok = CubeFile, Error = CubeError>>(
//     stream: S,
// ) -> eyre::Result<()> {
//     dbg!("i am going to start the download now!");
//     todo!()
// }
