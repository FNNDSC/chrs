use camino::Utf8PathBuf;
use clap::Parser;
use color_eyre::eyre;
use color_eyre::eyre::{bail, Context};
use fs_err::tokio::File;
use futures::TryStreamExt;
use futures::{StreamExt, TryFutureExt};
use std::path::Path;
use indicatif::HumanBytes;
use tokio::join;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_util::io::StreamReader;

use chris::search::Search;
use chris::types::PluginInstanceId;
use chris::{BasicFileResponse, Downloadable, EitherClient, LinkedModel, RoAccess, RoClient};

use crate::arg::{FeedOrPluginInstance, GivenDataNode};
use crate::credentials::Credentials;
use crate::file_transfer::{
    progress_bar_bytes, FileTransferError, FileTransferEvent, MultiFileTransferProgress,
};
use crate::files::decoder::MaybeChrisPathHumanCoder;

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

    /// Maximum number of concurrent HTTP requests
    #[clap(short = 'j', long, default_value_t = 4)]
    threads: usize,

    /// What to download.
    src: GivenDataNode,

    /// Directory where to download
    dst: Option<Utf8PathBuf>,
}

pub async fn download(credentials: Credentials, args: DownloadArgs) -> eyre::Result<()> {
    let (client, old, _) = credentials.get_client([args.src.as_arg_str()]).await?;
    let files = get_files_search(&client, args.src.clone(), old).await?;
    let size = download_files(client, files, args).await?;
    eprintln!("Downloaded: {}", HumanBytes(size));
    Ok(())
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
            Ok(logged_in.files().fname(path).search().basic().into_ro())
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

async fn download_files(
    client: EitherClient,
    files: Files,
    args: DownloadArgs,
) -> eyre::Result<u64> {
    let count = files.get_count().await?;
    if count == 0 {
        bail!("No files found")
    };
    if count == 1 {
        download_single_file(files, args).await
    } else {
        let ro_client = client.into_ro();
        download_many_files(&ro_client, files, args, count as u64).await
    }
}

/// Download one file, showing a file_transfer bar.
async fn download_single_file(files: Files, args: DownloadArgs) -> eyre::Result<u64> {
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
    Ok(only_file.object.fsize())
}

const SIZE_128_MIB: u64 = 134217728;

async fn download_many_files(
    ro_client: &RoClient,
    files: Files,
    args: DownloadArgs,
    count: u64,
) -> eyre::Result<u64> {
    let coder = MaybeChrisPathHumanCoder::new(ro_client, !args.no_titles);

    let (progress_tx, mut progress_rx) = unbounded_channel();
    let transfer_progress_loop = async {
        let mut transfer_progress = MultiFileTransferProgress::new(count, SIZE_128_MIB);
        while let Some(event) = progress_rx.recv().await {
            transfer_progress.update(event)
        }
        transfer_progress.total_size()
    };
    let download_loop = async move {
        // I am wrapped in an async move to drop progress_tx after all transfers are complete
        files
            .stream_connected()
            .enumerate()
            .map(|(i, r)| r.map(|f| (i, f, progress_tx.clone())))
            .map_err(FileTransferError::Cube)
            .try_for_each_concurrent(args.threads, download_with_events)
            .await
    };
    let (total_size, result) = join!(transfer_progress_loop, download_loop);
    result.map(|_| total_size).map_err(eyre::Error::new)
}

/// Download a single file while pushing events through a channel.
async fn download_with_events(
    (id, chris_file, ptx): (
        usize,
        LinkedModel<BasicFileResponse, RoAccess>,
        UnboundedSender<FileTransferEvent>,
    ),
) -> Result<(), FileTransferError> {
    let dst = chris_file.object.fname().as_str(); // TODO rename
    let dst_path = Path::new(dst);
    if let Some(parent_dirs) = dst_path.parent() {
        fs_err::tokio::create_dir_all(parent_dirs).await?;
    }
    let mut file = File::create(dst_path).await?;

    let stream = chris_file
        .stream()
        .await?
        .map_ok(|chunk| {
            ptx.send(FileTransferEvent::Chunk {
                id,
                delta: chunk.len() as u64,
            })
            .unwrap();
            chunk
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e));
    let mut reader = StreamReader::new(stream);
    ptx.send(FileTransferEvent::Start {
        id,
        name: chris_file.object.basename().to_string(),
        size: chris_file.object.fsize(),
    })
    .unwrap();
    tokio::io::copy(&mut reader, &mut file)
        .await
        .map(|_| ptx.send(FileTransferEvent::Done(id)).unwrap())
        .map_err(FileTransferError::IO)
}

//
// async fn download_files<S: TryStream<Ok = CubeFile, Error = CubeError>>(
//     stream: S,
// ) -> eyre::Result<()> {
//     dbg!("i am going to start the download now!");
//     todo!()
// }
