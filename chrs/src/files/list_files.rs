use crate::files::human_paths::MaybeRenamer;
use chris::filebrowser::FileBrowserPath;
use chris::ChrisClient;

pub(crate) async fn list_files(
    client: &ChrisClient,
    path: &FileBrowserPath,
    full: bool,
    depth: u16,
    namer: MaybeRenamer,
) -> anyhow::Result<()> {
    todo!()
}
