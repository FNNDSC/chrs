use crate::files::human_paths::MaybeNamer;
use anyhow::bail;
use async_recursion::async_recursion;
use async_stream::stream;
use chris::filebrowser::{FileBrowser, FileBrowserPath, FileBrowserView};
use chris::models::Downloadable;
use chris::ChrisClient;
use console::{style, StyledObject};
use futures::lock::Mutex;
use futures::TryStreamExt;
use indicatif::ProgressBar;
use std::sync::Arc;
use termtree::Tree;
use tokio::join;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

/// Show files in _ChRIS_ using the file browser API in a tree diagram.
pub(crate) async fn files_tree(
    client: &ChrisClient,
    path: &FileBrowserPath,
    full: bool,
    depth: u16,
    namer: MaybeNamer,
) -> anyhow::Result<()> {
    let fb = client.file_browser();
    match fb.browse(path).await? {
        None => bail!("Cannot find: {}", path),
        Some(v) => print_tree_from(&fb, v, full, depth, namer).await,
    }?;
    anyhow::Ok(())
}

async fn print_tree_from(
    fb: &FileBrowser,
    v: FileBrowserView,
    full: bool,
    depth: u16,
    mut namer: MaybeNamer,
) -> anyhow::Result<()> {
    let top_path = v.path().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let main = async move {
        let spinner = ProgressBar::new_spinner();
        let mut count = 0;
        while (rx.recv().await).is_some() {
            count += 1;
            spinner.set_message(format!("Getting information... {}", count));
        }
    };
    let tree_builder = construct(
        fb,
        tx,
        v,
        top_path,
        depth,
        full,
        DescentContext::Base,
        &mut namer,
    );
    let (_, tree) = join!(main, tree_builder);
    println!("{}", tree?);
    anyhow::Ok(())
}

/// Recursively construct a tree for a ChRIS directory path containing files.
#[async_recursion]
async fn construct(
    fb: &FileBrowser,
    tx: UnboundedSender<()>,
    v: FileBrowserView,
    folder_name: String,
    depth: u16,
    full: bool,
    context: DescentContext,
    namer: &mut MaybeNamer,
) -> anyhow::Result<Tree<StyledObject<String>>> {
    let root = style_folder(namer, v.path(), folder_name, context, full).await;
    if depth == 0 {
        return anyhow::Ok(root);
    }

    let maybe_subfolders = subfolders(fb, &v).await.map_err(anyhow::Error::msg)?;

    // fancy rust async stuff, don't mind me
    let stx = tx.clone();

    // namer is moved by generator, so we use Arc
    let descent = Arc::new(Mutex::new(namer));
    let arc = Arc::clone(&descent);
    let mut rn = arc.lock().await;
    let subtree_stream = stream! {
        for maybe in maybe_subfolders {
            if let Some((subfolder, child)) = maybe {
                yield construct(fb, stx.clone(), child, subfolder, depth - 1, full, context, *rn).await;
                // notify channel that we have done some work
                stx.send(()).unwrap();
            }
        }
    };
    let mut subtrees: Vec<Tree<StyledObject<String>>> = subtree_stream.try_collect().await?;

    let mut rn = descent.lock().await;
    let files = subfiles(v, *rn, full).await?;
    subtrees.extend(files);
    anyhow::Ok(root.with_leaves(subtrees))
}

/// Indicates what part of a CUBE (swift) file path we are looking at.
#[derive(Copy, Clone)]
enum DescentContext {
    /// Left-most base path, which is either a username or "SERVICES"
    Base,
    /// Second-from-the-left component, which is either "feed_N", "PACS", or "UPLOADS"
    Feed,
    /// A middle component of a plugin instance output file's fname
    /// after the feed and before the "data" folder.
    PluginInstances,
    /// A path which lacks a human-friendly name, e.g. PACS file, uploaded file.
    Data,
}

async fn style_folder(
    namer: &mut MaybeNamer,
    v: &FileBrowserPath,
    folder_name: String,
    context: DescentContext,
    full: bool,
) -> Tree<StyledObject<String>> {
    let display_name = if full {
        namer.rename(&v.clone().into()).await
    } else {
        folder_name // TODO context
    };
    Tree::new(style(display_name).bright().blue())
}

/// Get subfolders under a given filebrowser path. Returns 2-tuples of (name, object)
///
/// The FileBrowser API is susceptible to producing erroneous subfolder names
/// in the cases where path names contain the special character `,` because
/// `,` is used as a deliminiter.
async fn subfolders(
    fb: &FileBrowser,
    v: &FileBrowserView,
) -> Result<Vec<Option<(String, FileBrowserView)>>, String> {
    let subfolders_stream = stream! {
        for subfolder in v.subfolders() {
            let child_path = format!("{}/{}", v.path(), subfolder);
            yield fb.browse(&FileBrowserPath::from(child_path.as_str()))
                .await
                .map(|m| m.map(|child| (subfolder.to_string(), child)))
                .map_err(|_| format!("BUG: Invalid child path: {}", &child_path));
        }
    };
    subfolders_stream.try_collect().await
}

/// Get file names under a given filebrowser path and apply console output styling to them.
async fn subfiles(
    v: FileBrowserView,
    namer: &mut MaybeNamer,
    full: bool,
) -> Result<impl Iterator<Item = Tree<StyledObject<String>>>, reqwest::Error> {
    let namer = Arc::new(Mutex::new(namer));
    let thing = v.iter_files().try_filter_map(|file| {
        let arc = Arc::clone(&namer);
        async move {
            let mut rn = arc.lock().await;
            let namer = &mut *rn;
            Ok(Some(namer.rename(file.fname()).await))
        }
    });

    // calling collect so that we can use .map instead of streams
    let file_infos: Vec<String> = thing.try_collect().await?;
    let files = file_infos.into_iter().map(style).map(Tree::new);
    Ok(files)
}
