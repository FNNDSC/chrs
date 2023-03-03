use std::path::PathBuf;
use chris::common_types::Username;
use chris::CUBEAuth;
use clap::Parser;

#[derive(Parser)]
#[clap(about = "chrs mount proof-of-concept")]
struct Cli {
    /// CUBE filebrowser path
    path: String,
    /// mount point
    mountpoint: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    let chris = CUBEAuth::new(
         "https://cube.chrisproject.org/api/v1/".to_string().parse().unwrap(),
        Username::new("chris".to_string()),
        "chris1234".to_string()
    ).into_client().await?;

    Ok(())
}

