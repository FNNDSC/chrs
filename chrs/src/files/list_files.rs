use crate::files::fname_util::MaybeNamer;
use chris::ChrisClient;

pub(crate) async fn list_files(
    client: &ChrisClient,
    path: String,
    full: bool,
    depth: u16,
    namer: MaybeNamer,
) -> anyhow::Result<()> {
    todo!()
}
