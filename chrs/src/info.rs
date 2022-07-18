use chris::ChrisClient;
use console::style;

pub async fn cube_info(chris: &ChrisClient) -> anyhow::Result<()> {
    println!(
        "Logged into ChRIS {} as user {}",
        style(chris.url()).blue().underlined(),
        style(chris.username()).green()
    );
    anyhow::Ok(())
}
