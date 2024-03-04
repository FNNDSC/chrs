use chris::ChrisClient;

pub(crate) async fn get(client: &ChrisClient, url: &str) -> anyhow::Result<()> {
    let res = client.get(url).await?;
    println!("{}", res);
    Ok(())
}
