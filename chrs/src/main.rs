mod config;
mod upload;
mod login;

use std::path::PathBuf;
use std::str::FromStr;


use clap::{Parser, Subcommand};
use anyhow::{bail, Context, Ok, Result};

use chris::types::{Username, CUBEApiUrl};

#[derive(Parser)]
#[clap(
    author, version, about, long_about = None,
    propagate_version = false, disable_help_subcommand = true
)]
struct Cli {
    /// CUBE address
    #[clap(short, long)]
    address: Option<String>,

    /// account username
    #[clap(long)]
    username: Option<String>,

    /// account password
    #[clap(long)]
    password: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload local data to my ChRIS library
    Upload {
        /// Files and directories to upload
        #[clap(required = true)]
        files: Vec<PathBuf>,

        /// Path in swift to upload to
        #[clap(short, long, default_value_t=String::from(""))]
        path: String,
    },

    /// Remember ChRIS login account.
    ///
    /// Stores a username and authorization token for a given ChRIS API URL.
    Login {
        // it would be nice to have the --address, --username, ... duplicated here
    },

    /// Forget ChRIS login account
    Logout {

    }
}


#[tokio::main]
async fn main() -> Result<()> {
    let args: Cli = Cli::parse();

    let mut address: Option<CUBEApiUrl> = None;
    let mut username: Option<Username> = None;
    let password = args.password;

    if let Some(given_address) = args.address {
        address = Some(CUBEApiUrl::from_str(&*given_address)?);
    }
    if let Some(given_username) = args.username {
        username = Some(Username::from_str(&*given_username)?);
    }

    match &args.command {
        Commands::Upload { files, path } => {
            // upload(files, path)?;
            bail!("not implemented anymore");
        },
        Commands::Login {} => {
            login::cmd::login(address, username, password).await?;
        },
        Commands::Logout {} => {
            bail!("not implemented");
        },
    };
    Ok(())
}
