use super::coder_channel::DecodeChannel;
use crate::get_client::RoClient;
use crate::ls::options::WhatToPrint;
use async_recursion::async_recursion;
use chris::types::{FileBrowserPath, FileResourceFname};
use chris::FileBrowser;
use color_eyre::eyre::{eyre, Result};
use color_eyre::owo_colors::OwoColorize;
use futures::{pin_mut, StreamExt};

pub async fn ls_plain(
    client: &RoClient,
    path: &str,
    level: u16,
    full: bool,
    mut coder: DecodeChannel,
    what_to_print: WhatToPrint,
) -> Result<()> {
    let relative_parent = if full {
        None
    } else {
        Some(coder.decode(path.to_string()).await)
    };
    let was = ls_recursive(
        client.filebrowser(),
        path.into(),
        level,
        &relative_parent,
        &mut coder,
        what_to_print,
        Default::default(),
    )
    .await?;

    if !was.printed && was.had_subdirs {
        // future work: add the rest of chrs' arguments here too.
        let mut cmd: Vec<String> = std::env::args().collect();
        cmd.insert(cmd.len() - 1, "--show=folders".to_string());
        eprintln!(
            "Path contains subfolders but no files. To show directories, run `{}`",
            cmd.join(" ").bold()
        )
    }

    Ok(())
}

#[async_recursion]
async fn ls_recursive(
    fb: FileBrowser,
    path: FileBrowserPath,
    level: u16,
    relative_parent: &Option<String>,
    coder: &mut DecodeChannel,
    what_to_print: WhatToPrint,
    mut was: WasPrinted,
) -> Result<WasPrinted> {
    if level == 0 {
        return Ok(was);
    }
    let entry = fb
        .readdir(&path)
        .await?
        .ok_or_else(|| eyre!("Path not found: {}", path))?;

    was.had_subdirs = was.had_subdirs || !entry.subfolders().is_empty();

    if what_to_print.should_print_folders() {
        for subfolder in entry.absolute_subfolders() {
            print_path(coder, subfolder.take(), relative_parent).await?;
            was.printed = true;
        }
    }

    if what_to_print.should_print_files() {
        let iter_files = entry.iter_files();
        let files_stream = iter_files.stream();
        pin_mut!(files_stream);
        while let Some(file_result) = files_stream.next().await {
            let file_path: FileResourceFname = file_result?.into();
            print_path(coder, file_path.take(), relative_parent).await?;
            was.printed = true;
        }
    }

    // Recurse into subdirectories
    for subfolder in entry.absolute_subfolders() {
        let sub_was = ls_recursive(
            fb.clone(),
            subfolder,
            level - 1,
            relative_parent,
            coder,
            what_to_print,
            was,
        )
        .await?;
        was = was.reduce(sub_was);
    }
    Ok(was)
}

async fn print_path(
    coder: &mut DecodeChannel,
    fnamelike: String,
    relative_parent: &Option<String>,
) -> Result<()> {
    let relative_parent_len = relative_parent.as_ref().map(|s| s.len() + 1).unwrap_or(0);
    let ez_path = coder.decode(fnamelike).await;
    let rel_path = ez_path.get(relative_parent_len..).ok_or_else(|| {
        eyre!(
            "CUBE returned a file path \"{}\" which is not a subpath of parent {:?}",
            &ez_path,
            &relative_parent.as_slice()
        )
    })?;
    println!("{}", rel_path);
    Ok(())
}

#[derive(Default, Clone, Copy)]
struct WasPrinted {
    printed: bool,
    had_subdirs: bool,
}

impl WasPrinted {
    fn reduce(self, other: Self) -> Self {
        Self {
            printed: self.printed || other.printed,
            had_subdirs: self.had_subdirs || other.had_subdirs,
        }
    }
}
