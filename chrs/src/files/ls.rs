use crate::files::list_files::list_files;
use crate::files::tree::files_tree;
use chris::filebrowser::FileBrowserPath;
use chris::ChrisClient;
use crate::files::human_paths::MaybeRenamer;

pub(crate) async fn ls(
    client: &ChrisClient,
    path: &FileBrowserPath,
    level: u16,
    rename: bool,
    full: bool,
    tree: bool,
) -> anyhow::Result<()> {
    let namer = MaybeRenamer::new(client, rename);
    if tree {
        files_tree(client, path, full, level, namer).await
    } else {
        list_files(client, path, full, level, namer).await
    }
}
