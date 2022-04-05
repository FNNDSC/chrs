mod config;
mod login;

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{bail, Ok, Result};
use clap::{Parser, Subcommand};

use chris::types::{CUBEApiUrl, Username};

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
        /// Save token in plaintext instead of using keyring
        #[clap(long)]
        no_keyring: bool,
    },

    /// Forget ChRIS login account
    Logout {},
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
            println!("files={:?}, path={:?}", files, path);
            bail!("not implemented anymore");
        }
        Commands::Login { no_keyring } => {
            let backend = if *no_keyring {
                login::tokenstore::Backend::ClearText
            } else {
                login::tokenstore::Backend::Keyring
            };
            login::cmd::login(address, username, password, backend).await?;
        }
        Commands::Logout {} => {
            login::cmd::logout(address, username)?;
        }
    };
    Ok(())
}
