mod config;
mod login;
mod upload;

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};

use crate::config::ChrsConfig;
use crate::login::get_client::get_client;
use crate::upload::upload;
use chris::common_types::{CUBEApiUrl, Username};

#[derive(Parser)]
#[clap(
    author, version, about, long_about = None,
    propagate_version = false, disable_help_subcommand = true
)]
struct Cli {
    /// CUBE address
    #[clap(short, long, global = true)]
    address: Option<String>,

    /// account username
    #[clap(long, global = true)]
    username: Option<String>,

    /// account password
    #[clap(long, global = true)]
    password: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload local data to my ChRIS library
    Upload {
        /// Path prefix, i.e. subdir of <username>/uploads to upload to
        #[clap(short, long, default_value_t=String::from(""))]
        path: String,

        /// Files and directories to upload
        #[clap(required = true)]
        files: Vec<PathBuf>,
    },

    /// Remember ChRIS login account.
    ///
    /// Stores a username and authorization token for a given ChRIS API URL.
    Login {
        /// Save token in plaintext instead of using keyring
        #[clap(long)]
        no_keyring: bool,

        /// Take the password from stdin
        #[clap(long)]
        password_stdin: bool,
        // it would be nice to have the --address, --username, ... duplicated here
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
            let _client = get_client(address, username, password).await?;
            upload(files, path).await?;
        }
        Commands::Login {
            no_keyring,
            password_stdin,
        } => {
            let backend = if *no_keyring {
                login::tokenstore::Backend::ClearText
            } else {
                login::tokenstore::Backend::Keyring
            };
            login::cmd::login(address, username, password, backend, password_stdin).await?;
        }
        Commands::Logout {} => {
            login::cmd::logout(address, username)?;
        }
    };
    Ok(())
}
