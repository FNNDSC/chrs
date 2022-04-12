use anyhow::{Ok, Result};
use chris::ChrisClient;

pub(crate) async fn download(_client: &ChrisClient, _uri: &str) -> Result<()> {
    Ok(())
}
