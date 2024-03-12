use std::path::Path;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use color_eyre::eyre::eyre;
use color_eyre::{
    eyre,
    eyre::{bail, Context},
};
use fs_err::tokio::{File, OpenOptions};
use futures::{StreamExt, TryStreamExt};
use indicatif::HumanBytes;
use tokio::join;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedSender},
    Mutex,
};
use tokio_util::io::StreamReader;

use chris::search::Search;
use chris::types::{FileResourceFname, PluginInstanceId};
use chris::{
    BasicFileResponse, Downloadable, EitherClient, FeedResponse, LinkedModel,
    PluginInstanceResponse, RoAccess, RoClient,
};

use crate::arg::{FeedOrPluginInstance, GivenDataNode};
use crate::credentials::Credentials;
use crate::file_transfer::{
    progress_bar_bytes, FileTransferError, FileTransferEvent, MultiFileTransferProgress,
};
use crate::files::CoderChannel;
use crate::files::MaybeChrisPathHumanCoder;

#[derive(Parser)]
pub struct DownloadArgs {
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

    /// Maximum number of concurrent downloads
    #[clap(short = 'j', long, default_value_t = 4)]
    threads: usize,

    /// What to download.
    src: Option<GivenDataNode>,

    /// Directory where to download
    dst: Option<Utf8PathBuf>,
}

/// `chrs download` command
pub async fn download(credentials: Credentials, args: DownloadArgs) -> eyre::Result<()> {
    let (client, old, _) = credentials
        .get_client(args.src.as_ref().map(|g| g.as_arg_str()).as_slice())
        .await?;
    let src = args
        .src
        .clone()
        .or_else(|| old.map(|id| id.into()))
        .ok_or_else(|| eyre!("Missing operand"))?;
    let (files, dst, rel) = get_files_search(&client, src, old, args.dst.clone()).await?;
    let size = download_files(client, files, args, dst, rel).await?;
    eprintln!("Downloaded: {}", HumanBytes(size));
    Ok(())
}

type Files = Search<BasicFileResponse, RoAccess>;

/// Main implementation
async fn download_files(
    client: EitherClient,
    files: Files,
    args: DownloadArgs,
    dst: Utf8PathBuf,
    rel: String,
) -> eyre::Result<u64> {
    let count = files.get_count().await?;
    if count == 0 {
        bail!("No files found")
    };
    if count == 1 {
        download_single_file(files, args, dst).await
    } else {
        let ro_client = client.into_ro();
        download_many_files(&ro_client, files, args, dst, rel, count as u64).await
    }
}

/// Returns:
///
/// 0. Files to download
/// 1. download destination
/// 2. _CUBE_ relative path
async fn get_files_search(
    client: &EitherClient,
    given: GivenDataNode,
    old: Option<PluginInstanceId>,
    dst: Option<Utf8PathBuf>,
) -> eyre::Result<(Files, Utf8PathBuf, String)> {
    match client {
        EitherClient::LoggedIn(logged_in) => {
            if given.is_path() {
                let path = given.into_path(client, old).await?;
                let dst = dst.unwrap_or_else(|| basename(&path));
                let rel = path.to_string();
                Ok((
                    logged_in.files().fname(path).search().basic().into_ro(),
                    dst,
                    rel,
                ))
            } else {
                given
                    .into_or(client, old)
                    .await
                    .map(|fopi| choose_output_path(fopi, dst))
            }
        }
        EitherClient::Anon(_) => {
            let feed_or_plinst = given.into_or(client, old).await
                .wrap_err_with(|| "Cannot download arbitrary paths unless logged in due to a backend limitation. See https://github.com/FNNDSC/chrs/issues/32")?;
            Ok(choose_output_path(feed_or_plinst, dst))
        }
    }
}

/// Figure out what the _CUBE_ relative path is of a feed or plugin instance.
/// Also, choose a default download destination if necessary.
fn choose_output_path(
    feed_or_plinst: FeedOrPluginInstance<RoAccess>,
    dst: Option<Utf8PathBuf>,
) -> (Files, Utf8PathBuf, String) {
    match feed_or_plinst {
        FeedOrPluginInstance::Feed(f) => {
            let files = f.files();
            let dst = dst.unwrap_or_else(|| feed_name(&f.object));
            let rel = format!(
                "{}/feed_{}",
                f.object.creator_username.as_str(),
                f.object.id.0
            );
            (files, dst, rel)
        }
        FeedOrPluginInstance::PluginInstance(p) => {
            let files = p.files();
            let dst = dst.unwrap_or_else(|| plinst_title(&p.object));
            let rel = p.object.output_path;
            (files, dst, rel)
        }
    }
}

fn basename(path: &str) -> Utf8PathBuf {
    Utf8PathBuf::from(Utf8PathBuf::from(path).file_name().unwrap_or(path))
}

fn feed_name(feed: &FeedResponse) -> Utf8PathBuf {
    if feed.name.is_empty() {
        Utf8PathBuf::from(format!("feed_{}", feed.id.0))
    } else {
        Utf8PathBuf::from(&feed.name)
    }
}

fn plinst_title(plinst: &PluginInstanceResponse) -> Utf8PathBuf {
    if plinst.title.is_empty() {
        Utf8PathBuf::from(format!("{}_{}", plinst.plugin_name, plinst.id.0))
    } else {
        Utf8PathBuf::from(&plinst.title)
    }
}

/// Download one file, showing a file_transfer bar.
async fn download_single_file(
    files: Files,
    args: DownloadArgs,
    dst: Utf8PathBuf,
) -> eyre::Result<u64> {
    let only_file = files.get_only().await?;
    let existing_metadata = fs_err::tokio::metadata(&dst).await;
    if let Ok(metadata) = existing_metadata {
        if args.skip_existing && metadata.len() == only_file.object.fsize() {
            return Ok(0);
        }
    }
    let file = open(dst, &args).await?;
    let stream = only_file
        .stream()
        .await?
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e));
    let mut reader = StreamReader::new(stream);
    let pb = progress_bar_bytes(only_file.object.fsize());
    tokio::io::copy(&mut reader, &mut pb.wrap_async_write(file)).await?;
    Ok(only_file.object.fsize())
}

/// Opens a file with consideration of `--clobber`
async fn open(path: impl AsRef<Path>, args: &DownloadArgs) -> std::io::Result<File> {
    if args.clobber {
        File::create(path.as_ref()).await
    } else {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .await
    }
}

async fn download_many_files(
    ro_client: &RoClient,
    files: Files,
    args: DownloadArgs,
    dst: Utf8PathBuf,
    rel: String,
    count: u64,
) -> eyre::Result<u64> {
    let mut coder = MaybeChrisPathHumanCoder::new(ro_client, !args.no_titles);
    let renamed_rel = coder.decode(rel).await;
    let (coder_channel, coder_loop) = CoderChannel::create(coder);
    let mutex = Mutex::new(coder_channel);
    let coder_arc = Arc::new(mutex);

    let (progress_tx, mut progress_rx) = unbounded_channel();
    let transfer_progress_loop = async {
        let mut transfer_progress =
            MultiFileTransferProgress::new(count, crate::file_transfer::SIZE_128_MIB);
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
            .map(|(i, r)| {
                r.map(|f| {
                    (
                        i,
                        f,
                        progress_tx.clone(),
                        ManyCoder::new(Arc::clone(&coder_arc), &dst, &renamed_rel),
                    )
                })
            })
            .map_err(FileTransferError::Cube)
            .try_for_each_concurrent(args.threads, download_with_events)
            .await
    };
    let (total_size, result, _) = join!(transfer_progress_loop, download_loop, coder_loop);
    result.map(|_| total_size).map_err(eyre::Error::new)
}

struct ManyCoder<'a> {
    dst: &'a Utf8Path,
    rel: &'a str,
    coder: Arc<Mutex<CoderChannel>>,
}

impl<'a> ManyCoder<'a> {
    fn new(coder: Arc<Mutex<CoderChannel>>, dst: &'a Utf8Path, rel: &'a str) -> Self {
        Self { dst, rel, coder }
    }
    async fn name_output(self, fname: &FileResourceFname) -> Utf8PathBuf {
        let renamed = self.coder.lock().await.decode(fname.to_string()).await;
        join_output_name(&renamed, self.rel, self.dst)
    }
}

fn join_output_name(chris_fname: &str, chris_root: &str, dst: &Utf8Path) -> Utf8PathBuf {
    let rel = chris_fname
        .strip_prefix(chris_root)
        .map(|s| s.strip_prefix('/').unwrap_or(s))
        .unwrap_or(chris_root);
    dst.join(rel)
}

/// Download a single file while pushing events through a channel.
async fn download_with_events(
    (id, chris_file, ptx, coder): (
        usize,
        LinkedModel<BasicFileResponse, RoAccess>,
        UnboundedSender<FileTransferEvent>,
        ManyCoder<'_>,
    ),
) -> Result<(), FileTransferError> {
    let dst_path = coder.name_output(chris_file.object.fname()).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(
        "rudolph/feed_2/pl-dircopy_4/data/something.dat",
        "rudolph/feed_2/pl-dircopy_4/data",
        ".",
        "./something.dat"
    )]
    #[case(
        "rudolph/feed_2/pl-dircopy_4/data/subj1/abc.dcm",
        "rudolph/feed_2/pl-dircopy_4/data",
        ".",
        "./subj1/abc.dcm"
    )]
    #[case(
        "rudolph/feed_2/pl-dircopy_4/data/subj1/abc.dcm",
        "rudolph/feed_2/pl-dircopy_4/data",
        "output",
        "output/subj1/abc.dcm"
    )]
    fn test_join_output_name(
        #[case] fname: &str,
        #[case] root: &str,
        #[case] dst: &str,
        #[case] expected: &str,
    ) {
        let dst_path = Utf8Path::new(dst);
        let actual = join_output_name(fname, root, dst_path);
        let expected_path = Utf8PathBuf::from(expected);
        assert_eq!(actual, expected_path);
    }
}
