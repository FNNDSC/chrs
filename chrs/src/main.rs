mod constants;
mod executor;
mod feeds;
mod files;
mod get;
mod io_helper;
mod login;
mod pipeline_add;
mod plugin;
mod upload;
mod whoami;

use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgGroup, Parser, Subcommand};

use crate::feeds::list_feeds;
use crate::files::download::download;
use crate::files::ls;
use crate::get::get;
use crate::login::get_client::get_client;
use crate::pipeline_add::{add_pipeline, convert_pipeline};
use crate::plugin::{describe_plugin, run_latest};
use crate::upload::upload;
use crate::whoami::cube_info;
use chris::common_types::{CUBEApiUrl, Username};
use chris::models::{ComputeResourceName, PluginInstanceId, PluginName};
use login::saved::SavedLogins;

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

    /// Switch user
    Switch {},

    /// Show login information
    #[clap(visible_alias = "who")]
    Whoami {},

    /// List or search feeds
    Feeds {
        /// Number of feeds to get
        #[clap(short, long, default_value_t = 10)]
        limit: u32,
    },

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

        /// Do not rename folders with feed names and plugin instance titles
        #[clap(short, long)]
        raw: bool,

        /// Join contents of all "data" folders to the same output directory.
        ///
        /// Useful when trying to download sibling plugin instance outputs.
        #[clap(short, long, hide = true)]
        flatten: bool,

        /// Skip downloading of files which already exist on the filesystem,
        /// and where their file sizes match what is expected.
        #[clap(short = 'S', long)]
        skip_present: bool,

        /// What to download. Can either be a ChRIS Library files path or
        /// a files resource URL (such as a files search query or a feed
        /// files URL).
        src: String,

        /// Directory where to download
        dst: Option<PathBuf>,
    },

    /// Browse files in ChRIS
    Ls {
        /// tree-like output
        #[clap(short, long)]
        tree: bool,

        /// Maximum subdirectory depth
        #[clap(short = 'L', long, default_value_t = 2)]
        level: u16,

        /// Show full paths, which may be convenient for copy-paste
        #[clap(short, long)]
        full: bool,

        /// Do not rename folders with feed names and plugin instance titles
        #[clap(short, long)]
        raw: bool,

        /// (Swift) data path
        #[clap(default_value = "")]
        path: String,
    },

    //
    // /// Search for plugins and pipelines
    // Search {},
    //
    /// Get the parameters of a ChRIS plugin.
    PluginHelp {
        /// Name of a ChRIS plugin
        #[clap(required = true)]
        plugin_name: PluginName,
    },
    //
    /// Create a plugin instance by name.
    #[command(group(
    ArgGroup::new("cpu_request").required(false).args(["cpu", "cpu_limit"]),
    ))]
    RunLatest {
        /// CPU resource request, as number of CPU cores.
        #[clap(short = 'j', long, value_name = "N")]
        cpu: Option<u16>,

        /// CPU resource request.
        /// Format is xm where x is an integer in millicores.
        #[clap(long)]
        cpu_limit: Option<String>,

        /// Memory resource request.
        /// Format is xMi or xGi where x is an integer.
        #[clap(short, long)]
        memory_limit: Option<String>,

        /// GPU resource request.
        /// Number of GPUs to use for plugin instance.
        #[clap(short, long)]
        gpu_limit: Option<u32>,

        /// Number of workers resource request.
        /// Number of compute nodes for parallel job.
        #[clap(short, long)]
        number_of_workers: Option<u32>,

        /// Name of compute resource
        #[clap(short, long)]
        compute_resource_name: Option<ComputeResourceName>,

        /// Plugin instance title
        #[clap(short, long)]
        title: Option<String>,

        /// Parent plugin instance ID, which is the source of input files for ds-type plugins
        // TODO support accepting union type for convenience
        // e.g. feed URL, plugin instance URL, plugin instance title...
        #[clap(short, long)]
        previous_id: Option<u32>,

        /// Name of plugin to run
        #[clap(required = true)]
        plugin_name: PluginName,

        /// Plugin parameters
        parameters: Vec<String>,
    },

    /// Make an authenticated HTTP GET request
    #[clap(
        long_about = "Make an authenticated HTTP GET request (for debugging and advanced users)

The output of this subcommand can be piped into `jq`, e.g.

    chrs get https://cube.chrisproject.org/api/v1/ | jq"
    )]
    Get {
        /// CUBE resource URL
        url: String,
    },

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
        Commands::Feeds { limit } => {
            let client = get_client(address, username, password, vec![]).await?;
            list_feeds(&client, limit).await
        }

        Commands::Whoami {} => {
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
        Commands::Download {
            shorten,
            src,
            dst,
            raw,
            skip_present,
            flatten,
        } => {
            let client = get_client(address, username, password, vec![src.as_str()]).await?;
            download(
                &client,
                &src,
                dst.as_deref(),
                shorten,
                !raw,
                skip_present,
                flatten,
            )
            .await
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
        Commands::Switch {} => login::switch::switch_login(address, username),

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
        Commands::Ls {
            tree,
            level,
            full,
            raw,
            path,
        } => {
            let client = get_client(address, username, password, vec![]).await?;
            ls(&client, &path, level, !raw, full, tree).await
        }
        Commands::RunLatest {
            cpu,
            cpu_limit,
            memory_limit,
            gpu_limit,
            number_of_workers,
            compute_resource_name,
            plugin_name,
            previous_id,
            parameters,
            title,
        } => {
            let previous_id = previous_id.map(PluginInstanceId);
            let chris = get_client(address, username, password, vec![]).await?;
            run_latest(
                &chris,
                &plugin_name,
                &parameters,
                previous_id,
                cpu,
                cpu_limit,
                memory_limit,
                gpu_limit,
                number_of_workers,
                compute_resource_name,
                title,
            )
            .await
        }
        Commands::PluginHelp { plugin_name } => {
            let chris = get_client(address, username, password, vec![]).await?;
            describe_plugin(&chris, &plugin_name).await
        }
        Commands::Get { url } => {
            // TODO if logged into multiple CUBEs, get the right client
            let chris = get_client(address, username, password, vec![]).await?;
            get(&chris, &url).await
        }
    }
}
