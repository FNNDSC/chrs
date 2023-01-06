use crate::files::human_paths::MaybeNamer;
use crate::files::list_files::list_files;
use crate::files::tree::files_tree;
use chris::filebrowser::FileBrowserPath;
use chris::ChrisClient;

pub(crate) async fn ls(
    client: &ChrisClient,
    path: &FileBrowserPath,
    level: u16,
    rename: bool,
    full: bool,
    tree: bool,
) -> anyhow::Result<()> {
    let namer = MaybeNamer::new(client, rename);
    if tree {
        files_tree(client, path, full, level, namer).await
    } else {
        list_files(client, path, full, level, namer).await
    }
}
