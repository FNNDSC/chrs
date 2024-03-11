use async_walkdir::WalkDir;
use camino::Utf8PathBuf;
use clap::{builder::NonEmptyStringValueParser, Parser};
use color_eyre::eyre;
use color_eyre::eyre::{bail, eyre, WrapErr};
use color_eyre::owo_colors::OwoColorize;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use itertools::Itertools;
use tokio::try_join;
use tokio_util::codec::{BytesCodec, FramedRead};

use chris::{BaseChrisClient, ChrisClient, FeedRw, PluginInstanceRw, PluginRw};
use chris::types::{PluginInstanceId, PluginType};

use crate::credentials::{Credentials, NO_ARGS};
use crate::file_transfer::progress_bar_bytes;
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
    let (client, old, ui) = credentials.get_client(NO_ARGS).await?;
    if let Some(client) = client.logged_in() {
        upload_logged_in(client, old, ui, args).await
    } else {
        bail!("You must be logged in to upload files.")
    }
}

async fn upload_logged_in(
    client: ChrisClient,
    old: Option<PluginInstanceId>,
    ui: Option<UiUrl>,
    args: UploadArgs,
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
    } else if let Some(plinst) = plinsts.into_iter().next() {
        Some(plinst.feed().get().await?)
    } else {
        None
    };

    if let Some(feed) = feed {
        if let Some(note) = args.note {
            feed.note().set("Description", note).await?;
        }
        if let Some(ui) = ui {
            println!("{}", ui.feed_url_of(&feed.object))
        } else {
            println!("feed/{}", feed.object.name)
        }
    }
    Ok(())
}

async fn run_plugins(plugins: Vec<PluginRw>, mut previous_id: Option<PluginInstanceId>, upload_path: String) -> eyre::Result<Vec<PluginInstanceRw>> {
    let mut plinsts = Vec::with_capacity(plugins.len());
    for plugin in plugins {
        let (title, dir) = if matches!(plugin.object.plugin_type, PluginType::Fs | PluginType::Ts) {
            (Some("File upload from chrs"), Some(upload_path.as_str()))
        } else {
            (None, None)
        };
        let params = PluginParameters { title, dir, previous_id };
        dbg!(&plugin.object.name);
        let plinst = plugin.create_instance(dbg!(&params)).await?;
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
    previous_id: Option<PluginInstanceId>
}

async fn upload_all(
    client: &ChrisClient,
    files: Vec<Utf8PathBuf>,
    threads: usize,
) -> eyre::Result<String> {
    let base = create_upload_root_for(client);
    if files.len() == 1 {
        upload_single(client, files.into_iter().next().unwrap(), &base).await?;
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
    files: Vec<Utf8PathBuf>,
    base: &str,
    threads: usize,
) -> eyre::Result<()> {
    todo!()
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
            return Ok((Some(feed), Some(plinst_id)));
        }
    }
    Ok((None, old))
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

/// Collect all files in a set of paths.
async fn discover_files(paths: Vec<Utf8PathBuf>) -> Result<Vec<Utf8PathBuf>, std::io::Error> {
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
        .flat_map_unordered(None, WalkDir::new)
        .try_fold(vec![], file_entries_reducer)
        .await?;
    let files = either_file_or_dir.into_iter().filter_map(
        |(m, p)| {
            if m.is_file() {
                Some(p)
            } else {
                None
            }
        },
    );
    subdir_files.extend(files);
    Ok(subdir_files)
}

async fn file_entries_reducer(
    mut all_files: Vec<Utf8PathBuf>,
    entry: async_walkdir::DirEntry,
) -> Result<Vec<Utf8PathBuf>, std::io::Error> {
    let file_type = entry.file_type().await?;
    let path = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            eyre!("Path is invalid UTF-8: {:?}", entry.path()),
        )
    })?;
    if file_type.is_file() {
        all_files.push(path);
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
