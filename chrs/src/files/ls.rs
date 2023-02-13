use crate::files::fname_util::MaybeNamer;
use crate::files::list_files::list_files;
use crate::files::tree::files_tree;
use chris::ChrisClient;

pub(crate) async fn ls(
    client: &ChrisClient,
    path: &str,
    level: u16,
    rename: bool,
    full: bool,
    tree: bool,
) -> anyhow::Result<()> {
    let mut namer = MaybeNamer::new(client, rename);
    let path = namer.translate(path).await?;
    if tree {
        files_tree(client, path, full, level, namer).await
    } else {
        list_files(client, path, full, level, namer).await
    }
}
