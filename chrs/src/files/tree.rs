use std::sync::Arc;
use crate::files::human_paths::MaybeRenamer;
use anyhow::bail;
use async_recursion::async_recursion;
use async_stream::stream;
use chris::filebrowser::{FileBrowser, FileBrowserPath, FileBrowserView};
use chris::models::{Downloadable, DownloadableFile, FileResourceFname};
use chris::ChrisClient;
use console::{style, StyledObject};
use futures::{pin_mut, StreamExt, TryStreamExt};
use futures::lock::Mutex;
use indicatif::ProgressBar;
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
    namer: MaybeRenamer,
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
    mut namer: MaybeRenamer,
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
    let tree_builder = construct(fb, tx, v, top_path, full, depth, &mut namer);
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
    full: bool,
    depth: u16,
    namer: &mut MaybeRenamer,
) -> anyhow::Result<Tree<StyledObject<String>>> {
    let root = style_folder(v.path(), folder_name, full, namer).await;
    if depth == 0 {
        return anyhow::Ok(root);
    }

    let maybe_subfolders = subfolders(fb, &v).await.map_err(anyhow::Error::msg)?;

    // fancy rust async stuff, don't mind me
    let stx = tx.clone();

    // namer is moved by generator, so we use Arc
    let namer = Arc::new(Mutex::new(namer));
    let arc = Arc::clone(&namer);
    let mut rn = arc.lock().await;
    let subtree_stream = stream! {
        for maybe in maybe_subfolders {
            if let Some((subfolder, child)) = maybe {
                yield construct(fb, stx.clone(), child, subfolder, full, depth - 1, *rn).await;
                stx.send(()).unwrap();
            }
        }
    };
    let mut subtrees: Vec<Tree<StyledObject<String>>> = subtree_stream.try_collect().await?;

    let mut rn = namer.lock().await;
    let files = subfiles(&v, full, *rn).await?;
    subtrees.extend(files);
    anyhow::Ok(root.with_leaves(subtrees))
}

async fn style_folder(
    v: &FileBrowserPath,
    folder_name: String,
    full: bool,
    namer: &mut MaybeRenamer
) -> Tree<StyledObject<String>> {
    let display_name = if full {
        namer.rename(&v.clone().into()).await
    } else {
        folder_name
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
    v: &FileBrowserView,
    full: bool,
    namer: &mut MaybeRenamer,
) -> Result<impl Iterator<Item = Tree<StyledObject<String>>>, reqwest::Error> {
    wip_try_rename(v, full, namer).await;

    // calling collect so that we can use .map instead of streams
    let file_infos: Vec<DownloadableFile> = v.iter_files().try_collect().await?;
    let files = file_infos
        .into_iter()
        .map(which_name(full))
        .map(style)
        .map(Tree::new);
    Ok(files)
}

async fn wip_try_rename(
    v: &FileBrowserView,
    full: bool,
    namer: &mut MaybeRenamer,
) {
    let s = v.iter_files();
    pin_mut!(s);

    while let Some(res) = s.next().await {

        let x = match res {
            Ok(file) => {Ok(namer.rename(file.fname()).await)}
            Err(e) => {Err(e)}
        };
        dbg!(x);
    }
}

/// Resolves a helper function depending on the value for `full`.
fn which_name(full: bool) -> fn(DownloadableFile) -> String {
    if full {
        file2string
    } else {
        file2name
    }
}

fn file2string(f: DownloadableFile) -> String {
    f.fname().to_string()
}

fn file2name(f: DownloadableFile) -> String {
    let fname = f.fname().as_str();
    if let Some((_, basename)) = fname.rsplit_once('/') {
        return basename.to_string();
    }
    fname.to_string()
}
