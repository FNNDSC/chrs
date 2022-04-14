use anyhow::{bail, Ok, Result};
use async_recursion::async_recursion;
use async_stream::stream;
use chris::api::Downloadable;
use chris::filebrowser::{FileBrowser, FileBrowserPath, FileBrowserView};
use chris::ChrisClient;
use console::{style, StyledObject};
use futures::TryStreamExt;
use termtree::Tree;

/// Show files in _ChRIS_ using the file browser API in a tree diagram.
pub async fn ls(client: &ChrisClient, path: &FileBrowserPath, depth: u16) -> Result<()> {
    let fb = client.file_browser();
    match fb.browse(path).await? {
        None => bail!("Cannot find: {}", path),
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

    let subtree_stream = stream! {
        for maybe in maybes {
            if let Some((subfolder, child)) = maybe {
                yield construct(fb, child, subfolder, depth - 1).await;
            }
        }
    };
    let mut subtrees: Vec<Tree<StyledObject<String>>> = subtree_stream.try_collect().await?;

    let files_stream = stream! {
        for await file in v.iter_files() {
            yield Ok(Tree::new(style(file?.fname().to_string())))
        }
    };

    let files: Vec<Tree<StyledObject<String>>> = files_stream.try_collect().await?;
    subtrees.extend(files);

    Ok(root.with_leaves(subtrees))
}
