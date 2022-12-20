use chris::filebrowser::FileBrowserPath;
use chris::ChrisClient;
use crate::files::human_paths::MaybeRenamer;

pub(crate) async fn list_files(
    client: &ChrisClient,
    path: &FileBrowserPath,
    full: bool,
    depth: u16,
    namer: MaybeRenamer
) -> anyhow::Result<()> {
    todo!()
}
