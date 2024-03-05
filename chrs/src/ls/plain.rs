use super::coder_channel::DecodeChannel;
use crate::get_client::RoClient;
use async_recursion::async_recursion;
use chris::types::{FileBrowserPath, FileResourceFname};
use chris::FileBrowser;
use color_eyre::eyre::{eyre, Result};
use futures::{pin_mut, StreamExt};

pub async fn ls_plain(
    client: &RoClient,
    path: &str,
    level: u16,
    full: bool,
    mut coder: DecodeChannel,
) -> Result<()> {
    let relative_parent = if full {
        None
    } else {
        Some(coder.decode(path.to_string()).await)
    };
    ls_recursive(
        client.filebrowser(),
        path.into(),
        level,
        &relative_parent,
        &mut coder,
    )
    .await
}

#[async_recursion]
async fn ls_recursive(
    fb: FileBrowser,
    path: FileBrowserPath,
    level: u16,
    relative_parent: &Option<String>,
    coder: &mut DecodeChannel,
) -> Result<()> {
    if level == 0 {
        return Ok(());
    }
    let entry = fb
        .readdir(&path)
        .await?
        .ok_or_else(|| eyre!("Path not found: {}", path))?;

    // Print all files
    let relative_parent_len = relative_parent.as_ref().map(|s| s.len() + 1).unwrap_or(0);
    let iter_files = entry.iter_files();
    let files_stream = iter_files.stream();
    pin_mut!(files_stream);
    while let Some(file_result) = files_stream.next().await {
        let file_path: FileResourceFname = file_result?.into();
        let ez_path = coder.decode(file_path.take()).await;
        let ez_path_rel = ez_path.get(relative_parent_len..).ok_or_else(|| {
            eyre!(
                "CUBE returned a file path \"{}\" which is not a subpath of parent {:?}",
                &ez_path,
                &relative_parent.as_slice()
            )
        })?;
        println!("{}", ez_path_rel);
    }

    // Recurse into subdirectories
    for subfolder in entry.absolute_subfolders() {
        ls_recursive(fb.clone(), subfolder, level - 1, relative_parent, coder).await?;
    }

    Ok(())
}
