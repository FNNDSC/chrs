use std::path::PathBuf;

use async_walkdir::WalkDir;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{builder::NonEmptyStringValueParser, Parser};
use color_eyre::eyre;
use color_eyre::eyre::{bail, eyre, WrapErr};
use color_eyre::owo_colors::OwoColorize;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use itertools::Itertools;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::{join, try_join};
use tokio_util::codec::{BytesCodec, FramedRead};

use chris::types::{PluginInstanceId, PluginType};
use chris::{BaseChrisClient, ChrisClient, FeedRw, PluginInstanceRw, PluginRw};

use crate::credentials::{Credentials, NO_ARGS};
use crate::file_transfer::{progress_bar_bytes, FileTransferEvent, MultiFileTransferProgress};
use crate::login::UiUrl;
use crate::shlex::shlex_quote;

#[derive(Parser)]
pub struct UploadArgs {
    /// Feed name
    #[clap(short, long, value_parser = NonEmptyStringValueParser::new())]
    feed: Option<String>,

    /// Feed note
    #[clap(short, long, value_parser = NonEmptyStringValueParser::new())]
    note: Option<String>,

    /// Do not create a feed, just upload.
    #[clap(long, conflicts_with = "feed")]
    no_feed: bool,

    /// Do not run `pl-unstack-folders`
    #[clap(long)]
    no_unstack: bool,

    /// Maximum number of concurrent uploads
    #[clap(short = 'j', long, default_value_t = 4)]
    threads: usize,

    /// Paths to upload
    paths: Vec<Utf8PathBuf>,
}

/// `chrs upload` command
pub async fn upload(credentials: Credentials, args: UploadArgs) -> eyre::Result<()> {
    let config_path = credentials.config_path.clone();
    let (client, old, ui) = credentials.get_client(NO_ARGS).await?;
    if let Some(client) = client.logged_in() {
        upload_logged_in(client, old, ui, args, config_path).await
    } else {
        bail!("You must be logged in to upload files.")
    }
}

async fn upload_logged_in(
    client: ChrisClient,
    old: Option<PluginInstanceId>,
    ui: Option<UiUrl>,
    args: UploadArgs,
    config_path: Option<PathBuf>,
) -> eyre::Result<()> {
    let threads = args.threads;
    let input_paths = args.paths.clone();
    let get_cube_info = async {
        let (current_feed, previous_id) =
            find_existing_feed(&client, old, args.feed.as_deref()).await?;
        let plugins = find_plugins(&client, previous_id.is_some(), &args).await?;
        Ok::<_, eyre::Error>((current_feed, previous_id, plugins))
    };

    let ((current_feed, previous_id, plugins), files) = try_join!(
        get_cube_info,
        discover_files(input_paths).map_err(eyre::Error::new)
    )?;

    let upload_path = upload_all(&client, files, threads).await?;
    let plinsts = run_plugins(plugins, previous_id, upload_path).await?;

    let feed = if let Some(feed) = current_feed {
        Some(feed)
    } else if let Some(plinst) = plinsts.last() {
        Some(plinst.feed().get().await?)
    } else {
        None
    };
    if let Some(feed) = feed {
        if let Some(note) = args.note {
            feed.note().set("Description", note).await?;
        }
        if let Some(ui) = ui {
            eprintln!("{}", ui.feed_url_of(&feed.object))
        }
    }
    if let Some(plinst) = plinsts.last() {
        crate::login::set_cd(
            client.url(),
            client.username(),
            plinst.object.id,
            config_path,
        )?;
        println!("plugininstance/{}", plinst.object.id.0)
    }
    Ok(())
}

async fn run_plugins(
    plugins: Vec<PluginRw>,
    mut previous_id: Option<PluginInstanceId>,
    upload_path: String,
) -> eyre::Result<Vec<PluginInstanceRw>> {
    let mut plinsts = Vec::with_capacity(plugins.len());
    for plugin in plugins {
        let (title, dir) = if matches!(plugin.object.plugin_type, PluginType::Fs | PluginType::Ts) {
            (Some("File upload from chrs"), Some(upload_path.as_str()))
        } else {
            (None, None)
        };
        let params = PluginParameters {
            title,
            dir,
            previous_id,
        };
        let plinst = plugin.create_instance(&params).await?;
        previous_id = Some(plinst.object.id);
        plinsts.push(plinst);
    }
    Ok(plinsts)
}

#[derive(Debug, serde::Serialize)]
struct PluginParameters<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dir: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_id: Option<PluginInstanceId>,
}

async fn upload_all(
    client: &ChrisClient,
    files: Vec<DiscoveredFile>,
    threads: usize,
) -> eyre::Result<String> {
    let base = create_upload_root_for(client);
    if files.len() == 1 {
        upload_single(client, files.into_iter().next().unwrap().path, &base).await?;
    } else {
        upload_multiple(client, files, &base, threads).await?;
    }
    Ok(base)
}

/// Upload a single file with a progress bar.
async fn upload_single(client: &ChrisClient, file: Utf8PathBuf, base: &str) -> eyre::Result<()> {
    let file_name = file.file_name().unwrap_or(file.as_str()).to_string();
    let upload_name = format!("{}/{}", base, file_name);
    let content_length = fs_err::tokio::metadata(&file).await?.len();
    let open_file = fs_err::tokio::File::open(&file).await?;
    let pb = progress_bar_bytes(content_length);
    let stream = FramedRead::new(pb.wrap_async_read(open_file), BytesCodec::new());
    client
        .upload_stream(stream, file_name, upload_name, content_length)
        .await?;
    Ok(())
}

/// Upload multiple files with progress bars.
async fn upload_multiple(
    client: &ChrisClient,
    files: Vec<DiscoveredFile>,
    base: &str,
    threads: usize,
) -> eyre::Result<()> {
    let (tx, mut rx) = unbounded_channel();
    let total = files.len() as u64;
    let transfer_progress_loop = async {
        let mut transfer_progress =
            MultiFileTransferProgress::new(total, crate::file_transfer::SIZE_128_MIB);
        while let Some(event) = rx.recv().await {
            transfer_progress.update(event)
        }
    };
    let upload_loop = async move {
        // I am wrapped in an async move to drop tx after all transfers are complete
        futures::stream::iter(files)
            .enumerate()
            .map(Ok::<_, chris::errors::FileIOError>)
            .try_for_each_concurrent(threads, |(i, file)| {
                upload_with_events(client, base, file, i, tx.clone())
            })
            .await
    };
    let (_, result) = join!(transfer_progress_loop, upload_loop);
    result.map_err(eyre::Error::new)
}

async fn upload_with_events(
    client: &ChrisClient,
    base: &str,
    file: DiscoveredFile,
    id: usize,
    tx: UnboundedSender<FileTransferEvent>,
) -> Result<(), chris::errors::FileIOError> {
    let file_name = file
        .path
        .file_name()
        .unwrap_or(file.path.as_str())
        .to_string();
    let rel = pathdiff::diff_utf8_paths(&file.path, &file.src)
        .map(|p| p.to_string())
        .unwrap_or_else(|| file.path.to_string());
    let upload_name = format!("{}/{}", base, rel);
    let content_length = fs_err::tokio::metadata(&file.path).await?.len();
    let open_file = fs_err::tokio::File::open(&file.path).await?;
    let chunk_tx = tx.clone();
    let stream = FramedRead::new(open_file, BytesCodec::new()).map_ok(move |chunk| {
        chunk_tx
            .send(FileTransferEvent::Chunk {
                id,
                delta: chunk.len() as u64,
            })
            .unwrap();
        chunk
    });
    tx.send(FileTransferEvent::Start {
        id,
        name: file_name.to_string(),
        size: content_length,
    })
    .unwrap();
    client
        .upload_stream(stream, file_name, upload_name, content_length)
        .await?;
    tx.send(FileTransferEvent::Done(id)).unwrap();
    Ok(())
}

fn create_upload_root_for(client: &ChrisClient) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    format!(
        "{}/uploads/chrs-upload-tmp-{}",
        client.username().as_str(),
        now
    )
}

async fn find_existing_feed(
    client: &ChrisClient,
    old: Option<PluginInstanceId>,
    name: Option<&str>,
) -> eyre::Result<(Option<FeedRw>, Option<PluginInstanceId>)> {
    if let Some(name) = name {
        if let Some(feed) = get_feed_by_name(client, name).await? {
            let plinst_id = get_plinst_of_feed(client, &feed).await?;
            // Will add to specified feed
            Ok((Some(feed), Some(plinst_id)))
        } else {
            // Will create a new feed
            Ok((None, None))
        }
    } else {
        // Will add to current feed
        Ok((None, old))
    }
}

async fn get_feed_by_name(client: &ChrisClient, name: &str) -> eyre::Result<Option<FeedRw>> {
    let feeds: Vec<_> = client
        .feeds()
        .name_exact(name)
        .search()
        .page_limit(2)
        .max_items(2)
        .stream_connected()
        .try_collect()
        .await?;
    if feeds.len() > 1 {
        bail!(
            "Multiple feeds found. Hint: run `{}` and specify feed name by feed/{}",
            format!("chrs list {}", shlex_quote(name)).bold(),
            "ID".bold().green()
        )
    }
    Ok(feeds.into_iter().next())
}

/// Try to get the root plugin instance of a feed. However, since we can't get this from the API
/// directly (see https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/541), instead we will:
///
/// 1. get the most recent 20 plugin instances.
/// 2. If the root is found in the 20 plugin instances, then return it.
/// 3. Otherwise, return the most recent plugin instance.
async fn get_plinst_of_feed(client: &ChrisClient, feed: &FeedRw) -> eyre::Result<PluginInstanceId> {
    let plinsts: Vec<_> = client
        .plugin_instances()
        .feed_id(feed.object.id)
        .search()
        .page_limit(20)
        .max_items(20)
        .stream()
        .try_collect()
        .await?;
    plinsts
        .into_iter()
        .find_or_first(|p| p.plugin_type == PluginType::Fs)
        .map(|p| p.id)
        .ok_or_else(|| eyre!("Feed does not contain plugin instances. This is a CUBE bug."))
}

/// A file to be uploaded.
struct DiscoveredFile {
    /// The path to the file
    path: Utf8PathBuf,
    /// Positional argument path which this file was discovered under
    src: Utf8PathBuf,
}

/// Collect all files in a set of paths.
async fn discover_files(paths: Vec<Utf8PathBuf>) -> Result<Vec<DiscoveredFile>, std::io::Error> {
    let either_file_or_dir: Vec<(std::fs::Metadata, Utf8PathBuf)> = futures::stream::iter(paths)
        .map(|p| async move { fs_err::tokio::metadata(&p).await.map(|m| (m, p)) })
        .map(Ok::<_, std::io::Error>)
        .try_buffer_unordered(100)
        .try_collect()
        .await?;
    let dirs = either_file_or_dir
        .iter()
        .filter_map(|(m, p)| if m.is_dir() { Some(p) } else { None });
    let mut subdir_files = futures::stream::iter(dirs)
        .flat_map_unordered(None, |src| {
            WalkDir::new(src).map_ok(|entry| (src.as_path(), entry))
        })
        .try_fold(vec![], file_entries_reducer)
        .await?;
    let files = either_file_or_dir
        .into_iter()
        .filter_map(|(metadata, path)| {
            if metadata.is_file() {
                Some(DiscoveredFile {
                    src: path.clone(),
                    path,
                })
            } else {
                None
            }
        });
    subdir_files.extend(files);
    Ok(subdir_files)
}

async fn file_entries_reducer(
    mut all_files: Vec<DiscoveredFile>,
    (src, entry): (&Utf8Path, async_walkdir::DirEntry),
) -> Result<Vec<DiscoveredFile>, std::io::Error> {
    let file_type = entry.file_type().await?;
    let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            eyre!("Path is invalid UTF-8: {:?}", entry.path()),
        )
    })?;
    if file_type.is_file() {
        all_files.push(DiscoveredFile {
            src: src.to_path_buf(),
            path,
        });
    }
    Ok(all_files)
}

async fn find_plugins(
    client: &ChrisClient,
    existing: bool,
    args: &UploadArgs,
) -> eyre::Result<Vec<PluginRw>> {
    if args.no_feed {
        return Ok(Vec::with_capacity(0));
    }
    let mut plugins = Vec::with_capacity(2);
    let (first_plugin_name, first_plugin_version) = if existing {
        ("pl-tsdircopy", "1.2.1")
    } else {
        ("pl-dircopy", "2.1.2")
    };
    let first_plugin = client
        .plugin()
        .name_exact(first_plugin_name)
        .version(first_plugin_version)
        .search()
        .get_only()
        .await
        .wrap_err_with(|| {
            format!(
                "Plugin {}@{} not found",
                first_plugin_name, first_plugin_version
            )
        })?;
    plugins.push(first_plugin);
    if !args.no_unstack {
        let second_plugin = client
            .plugin()
            .name_exact("pl-unstack-folders")
            .version("1.0.0")
            .search()
            .get_only()
            .await
            .wrap_err_with(|| "pl-unstack-folders@1.0.0 not found")?;
        plugins.push(second_plugin);
    }
    Ok(plugins)
}
