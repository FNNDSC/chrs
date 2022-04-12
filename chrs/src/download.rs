use anyhow::{Ok, Result};
use chris::api::AnyFilesUrl;
use chris::ChrisClient;

pub(crate) async fn download(_client: &ChrisClient, url: &AnyFilesUrl) -> Result<()> {
    Ok(())
}
