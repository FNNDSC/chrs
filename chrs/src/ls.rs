use anyhow::{bail, Ok, Result};
use async_recursion::async_recursion;
use async_stream::stream;
use chris::api::{Downloadable, DownloadableFile};
use chris::filebrowser::{FileBrowser, FileBrowserPath, FileBrowserView};
use chris::ChrisClient;
use console::{style, StyledObject};
use futures::TryStreamExt;
use termtree::Tree;
use tokio::sync::mpsc;
use indicatif::ProgressBar;
use tokio::sync::mpsc::UnboundedSender;
use tokio::join;

/// Show files in _ChRIS_ using the file browser API in a tree diagram.
pub async fn ls(
    client: &ChrisClient,
    path: &FileBrowserPath,
    full: bool,
    depth: u16,
) -> Result<()> {
    let fb = client.file_browser();
    match fb.browse(path).await? {
        None => bail!("Cannot find: {}", path),
        Some(v) => {
            print_tree_from(&fb, v, full, depth).await
        }
    }?;
    Ok(())
}

async fn print_tree_from(
    fb: &FileBrowser,
    v: FileBrowserView,
    full: bool,
    depth: u16,
) -> Result<()> {
    let top_path = v.path().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let main = async move {
        let spinner = ProgressBar::new_spinner();
        let mut count = 0;
        while let Some(_) = rx.recv().await {
            count = count + 1;
            spinner.set_message(format!("Getting information... {}", count));
        }
    };
    let tree_builder = construct(&fb, tx, v, top_path, full, depth);
    let (_, tree) = join!(main, tree_builder);
    println!("{}", tree?);
    Ok(())
}

/// Recursively construct a tree for a ChRIS directory path containing files.
#[async_recursion]
async fn construct(
    fb: &FileBrowser,
    tx: UnboundedSender<()>,
    v: FileBrowserView,
    current_path: String,
    full: bool,
    depth: u16,
) -> Result<Tree<StyledObject<String>>> {
    let root = Tree::new(style(current_path).bright().blue());
    if depth == 0 {
        return Ok(root);
    }

    let subfolders = v.subfolders();
    let path = v.path();
    let subfolders_stream = stream! {
        for subfolder in subfolders {
            let child_path = format!("{}/{}", path, subfolder);
            yield fb.browse(&FileBrowserPath::new(&child_path))
                .await
                .map(|m| m.map(|child| (subfolder.to_string(), child)))
                .map_err(|_| format!("BUG: Invalid child path: {}", &child_path));
        }
    };
    let maybes: Vec<Option<(String, FileBrowserView)>> = subfolders_stream
        .try_collect()
        .await
        .map_err(anyhow::Error::msg)?;

    let stx = tx.clone();
    let subtree_stream = stream! {
        for maybe in maybes {
            if let Some((subfolder, child)) = maybe {
                yield construct(fb, stx.clone(), child, subfolder, full, depth - 1).await;
                stx.send(()).unwrap();
            }
        }
    };
    let mut subtrees: Vec<Tree<StyledObject<String>>> = subtree_stream.try_collect().await?;

    let files_stream = stream! {
        for await file in v.iter_files() {
            yield file;
        }
    };
    let file_infos: Vec<DownloadableFile> = files_stream.try_collect().await?;
    let files = file_infos
        .into_iter()
        .map(namer(full))
        .map(style)
        .map(Tree::new);
    subtrees.extend(files);
    Ok(root.with_leaves(subtrees))
}

fn namer(full: bool) -> fn(DownloadableFile) -> String {
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
