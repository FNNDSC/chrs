mod config;
mod constants;
mod login;
mod pipeline_add;
mod upload;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::config::ChrsConfig;
use crate::login::get_client::get_client;
use crate::pipeline_add::{add_pipeline, convert_pipeline};
use crate::upload::upload;
use chris::common_types::{CUBEApiUrl, Username};

#[derive(Parser)]
#[clap(
    version,
    about = "Manage ChRIS files, plugins, and pipelines.",
    propagate_version = false,
    disable_help_subcommand = true
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
    /// Upload files to my ChRIS library
    Upload {
        /// Path prefix, i.e. subdir of <username>/uploads to upload to
        #[clap(short, long, default_value_t=String::from(""))]
        path: String,

        /// Files and directories to upload
        #[clap(required = true)]
        files: Vec<PathBuf>,
    },
    //
    // /// Download files from ChRIS
    // Download {},
    //
    // /// Search for files in ChRIS
    // Ls {},
    //
    // /// Search for plugins and pipelines
    // Search {},
    //
    // /// Get information about a ChRIS resource
    // Describe {},
    //
    // /// Run plugins and pipelines
    // Run {},
    /// Remember login account
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

    /// Forget login
    Logout {},

    /// Work with file-representation of pipelines
    #[clap(subcommand)]
    PipelineFile(PipelineFile),
}

#[derive(Subcommand)]
enum PipelineFile {
    // /// Export pipeline to a file
    // Export,
    //
    // /// Render pipeline file as a tree
    // Tree,
    /// Upload a pipeline to ChRIS
    Add {
        /// File representation of a pipeline.
        /// Can be either JSON (canonical) or YAML (ChRIS RFC #2).
        file: PathBuf, // TODO
                       // name
                       // authors
                       // category
                       // description
                       // unlocked
                       // locked
    },

    /// Convert between pipeline file formats (usually for debugging).
    ///
    /// Supported formats: JSON, YAML.
    Convert {
        /// If output type is JSON, serialize `plugin_tree` as an object
        /// instead of a string.
        #[clap(short, long)]
        expand: bool,

        /// Source pipeline file.
        src: PathBuf,
        /// Output file.
        dst: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Cli = Cli::parse();

    let mut address: Option<CUBEApiUrl> = None;
    let mut username: Option<Username> = None;
    let password = args.password;

    if let Some(given_address) = args.address {
        address = Some(CUBEApiUrl::new(&*given_address)?);
    }
    if let Some(given_username) = args.username {
        username = Some(Username::new(&*given_username));
    }

    match &args.command {
        Commands::Upload { files, path } => {
            let client = get_client(address, username, password).await?;
            upload(&client, files, path).await
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
            login::cmd::login(address, username, password, backend, password_stdin).await
        }
        Commands::Logout {} => login::cmd::logout(address, username),
        Commands::PipelineFile(pf_command) => {
            match pf_command {
                // PipelineFile::Export => { bail!("not implemented") }
                // PipelineFile::Tree => { bail!("not implemented") }
                PipelineFile::Add { file } => {
                    let client = get_client(address, username, password).await?;
                    add_pipeline(&client, file).await
                }
                PipelineFile::Convert { expand, src, dst } => {
                    convert_pipeline(expand, src, dst).await
                }
            }
        }
    }
}
