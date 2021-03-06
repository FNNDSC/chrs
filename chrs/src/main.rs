mod constants;
mod download;
mod executor;
mod files_tree;
mod info;
mod login;
mod pipeline_add;
mod upload;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::download::download;
use crate::files_tree::files_tree;
use crate::info::cube_info;
use crate::login::get_client::get_client;
use crate::pipeline_add::{add_pipeline, convert_pipeline};
use crate::upload::upload;
use chris::common_types::{CUBEApiUrl, Username};
use chris::filebrowser::FileBrowserPath;
use login::config::ChrsConfig;

#[derive(Parser)]
#[clap(
    version,
    about = "Manage ChRIS files and run pipelines.",
    propagate_version = false,
    disable_help_subcommand = true
)]
struct Cli {
    /// ChRIS backend URL
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
    /// Get information about the ChRIS backend.
    Info {},

    /// Upload files and run workflows
    Upload {
        /// Path prefix, i.e. subdir of <username>/uploads to upload to
        #[clap(short='P', long, default_value_t=String::from(""))]
        path: String,

        /// Create a feed with a name
        #[clap(short, long)]
        feed: Option<String>,

        /// Run a pipeline
        #[clap(short = 'p', long)]
        pipeline: Option<String>,

        /// Files and directories to upload
        #[clap(required = true)]
        files: Vec<PathBuf>,
    },

    /// Download files from ChRIS
    Download {
        /// Save files from under plugin instances' "data" subdirectory at
        /// the top-level, instead of under the nested parent directory.
        ///
        /// May be repeated to handle cases where the `data` subdirectory
        /// is deeply nested under parent `data` subdirectoies, e.g. `-sssss`.
        #[clap(short, long, action = clap::ArgAction::Count)]
        shorten: u8,

        /// What to download. Can either be a ChRIS Library files path or
        /// a files resource URL (such as a files search query or a feed
        /// files URL).
        src: String,

        /// Directory where to download
        #[clap(default_value = ".")]
        dst: PathBuf,
    },

    /// Browse files in ChRIS
    Tree {
        /// Maximum subdirectory depth
        #[clap(short = 'L', long, default_value_t = 2)]
        level: u16,

        /// Show full paths, which may be convenient for copy-paste
        #[clap(short, long)]
        full: bool,

        /// (Swift) data path
        #[clap(default_value = "")]
        path: FileBrowserPath,
    },

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
        address = Some(CUBEApiUrl::try_from(given_address)?);
    }
    if let Some(given_username) = args.username {
        username = Some(Username::new(given_username));
    }

    match args.command {
        Commands::Info {} => {
            let client = get_client(address, username, password, vec![]).await?;
            cube_info(&client).await
        }

        Commands::Upload {
            files,
            feed,
            pipeline,
            path,
        } => {
            let client = get_client(address, username, password, vec![]).await?;
            upload(&client, &files, &path, feed, pipeline).await
        }
        Commands::Download { shorten, src, dst } => {
            let client = get_client(address, username, password, vec![src.as_str()]).await?;
            download(&client, &src, &dst, shorten).await
        }
        Commands::Login {
            no_keyring,
            password_stdin,
        } => {
            let backend = if no_keyring {
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
                    let client = get_client(address, username, password, vec![]).await?;
                    add_pipeline(&client, &file).await
                }
                PipelineFile::Convert { expand, src, dst } => {
                    convert_pipeline(expand, &src, &dst).await
                }
            }
        }
        Commands::Tree { level, full, path } => {
            let client = get_client(address, username, password, vec![]).await?;
            files_tree(&client, &path, full, level).await
        }
    }
}
