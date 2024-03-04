use super::config::LsConfig;
use crate::get_client::{get_client, Credentials};
use color_eyre::eyre::Result;

pub async fn ls(
    credentials: Credentials,
    level: Option<u16>,
    path: Option<String>,
    retries: Option<u32>,
    config: LsConfig,
) -> Result<()> {
    let (client, pid) = get_client(credentials, path.as_slice(), retries).await?;
    let public_client = client.into_public();
    println!("i am the ls command");
    Ok(())
}
