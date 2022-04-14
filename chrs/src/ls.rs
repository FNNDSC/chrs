use anyhow::{bail, Context, Ok, Result};
use async_recursion::async_recursion;
use async_stream::stream;
use chris::filebrowser::{FileBrowser, FileBrowserPath, FileBrowserView};
use chris::api::Downloadable;
use chris::ChrisClient;
use console::{style, StyledObject};
use futures::{pin_mut, StreamExt, TryStreamExt};
use termtree::Tree;

/// Show files in _ChRIS_ using the file browser API in a tree diagram.
pub async fn ls(client: &ChrisClient, path: &FileBrowserPath, depth: u16) -> Result<()> {
    let fb = client.file_browser();
    match fb.browse(path).await? {
        None => bail!("Not a directory: {}", path),
        Some(v) => {
            let top_path = v.path().to_string();
            println!("{}", construct(&fb, v, top_path, depth).await?);
            Ok(())
        }
    }?;
    Ok(())
}

/// construct a tree for a ChRIS directory path containing files.
#[async_recursion]
async fn construct(
    fb: &FileBrowser,
    v: FileBrowserView,
    current_path: String,
    depth: u16,
) -> Result<Tree<StyledObject<String>>> {
    let root = Tree::new(style(current_path).bright().blue());
    if depth == 0 {
        return Ok(root);
    }

    let mut futures = Vec::new();
    for subfolder in v.subfolders() {
        let child_path = format!("{}/{}", v.path(), subfolder);
        let child = fb
            .browse(&FileBrowserPath::new(&child_path))
            .await
            .with_context(|| {
                format!(
                    "Invalid child path: {}\n{}",
                    &child_path,
                    style(
                        "This API is not capable of handling paths which contain commas. \
                    See https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/384"
                    )
                    .yellow()
                )
            })?;
        if let Some(c) = child {
            futures.push(construct(&fb, c, subfolder.to_string(), depth - 1));
        }
    }

    let mut leaves = Vec::with_capacity(futures.len());
    for future in futures {
        leaves.push(future.await?);
    }

    let files_iter = v.iter_files();
    pin_mut!(files_iter);
    while let Some(file) = files_iter.next().await {
        leaves.push(Tree::new(style(file?.fname().to_string())));
    }

    Ok(root.with_leaves(leaves))
}
