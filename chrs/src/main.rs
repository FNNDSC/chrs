// mod constants;
// mod executor;
// mod feeds;
// mod files;
// mod get;
// mod io_helper;
mod login;
// mod pipeline_add;
// mod plugin;
// mod upload;
// mod whoami;

use clap::{ArgGroup, Parser, Subcommand};

// use crate::feeds::list_feeds;
// use crate::files::download::download;
// use crate::files::ls;
// use crate::get::get;
// use crate::login::get_client::get_client;
// use crate::pipeline_add::{add_pipeline, convert_pipeline};
// use crate::plugin::{describe_plugin, run_latest};
// use crate::upload::upload;
// use crate::whoami::cube_info;
use chris::types::{CubeUrl, Username};
// use chris::models::data::{ComputeResourceName, PluginInstanceId, PluginName};
// use login::saved::SavedLogins;

#[derive(Parser)]
#[clap(
    version,
    about = "ChRIS Research Integration System -- command line client",
    propagate_version = false,
    disable_help_subcommand = true
)]
struct Cli {
    /// ChRIS backend API URL
    #[clap(short, long, global = true)]
    cube: Option<CubeUrl>,

    /// account username
    #[clap(long, global = true)]
    username: Option<String>,

    /// account password
    #[clap(long, global = true)]
    password: Option<String>,

    /// authorization token
    #[clap(long, global = true)]
    token: Option<String>,

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
    },
    //
    // /// Forget login
    // Logout {},
    //
    // /// Switch user
    // Switch {},
    /// Show login information
    Whoami {},
    //
    //     /// List or search feeds
    //     Feeds {
    //         /// Number of feeds to get
    //         #[clap(short, long, default_value_t = 10)]
    //         limit: u32,
    //     },
    //
    //     /// Upload files and run workflows
    //     Upload {
    //         /// Path prefix, i.e. subdir of <username>/uploads to upload to
    //         #[clap(short='P', long, default_value_t=String::from(""))]
    //         path: String,
    //
    //         /// Create a feed with a name
    //         #[clap(short, long)]
    //         feed: Option<String>,
    //
    //         /// Run a pipeline
    //         #[clap(short = 'p', long)]
    //         pipeline: Option<String>,
    //
    //         /// Files and directories to upload
    //         #[clap(required = true)]
    //         files: Vec<PathBuf>,
    //     },
    //
    //     /// Download files from ChRIS
    //     Download {
    //         /// Save files from under plugin instances' "data" subdirectory at
    //         /// the top-level, instead of under the nested parent directory.
    //         ///
    //         /// May be repeated to handle cases where the `data` subdirectory
    //         /// is deeply nested under parent `data` subdirectoies, e.g. `-sssss`.
    //         #[clap(short, long, action = clap::ArgAction::Count)]
    //         shorten: u8,
    //
    //         /// Do not rename folders with feed names and plugin instance titles
    //         #[clap(short, long)]
    //         raw: bool,
    //
    //         /// Join contents of all "data" folders to the same output directory.
    //         ///
    //         /// Useful when trying to download sibling plugin instance outputs.
    //         #[clap(short, long, hide = true)]
    //         flatten: bool,
    //
    //         /// Skip downloading of files which already exist on the filesystem,
    //         /// and where their file sizes match what is expected.
    //         #[clap(short = 'S', long)]
    //         skip_present: bool,
    //
    //         /// What to download. Can either be a ChRIS Library files path or
    //         /// a files resource URL (such as a files search query or a feed
    //         /// files URL).
    //         src: String,
    //
    //         /// Directory where to download
    //         dst: Option<PathBuf>,
    //     },
    //
    //     /// Browse files in ChRIS
    //     Ls {
    //         /// tree-like output
    //         #[clap(short, long)]
    //         tree: bool,
    //
    //         /// Maximum subdirectory depth
    //         #[clap(short = 'L', long, default_value_t = 2)]
    //         level: u16,
    //
    //         /// Show full paths, which may be convenient for copy-paste
    //         #[clap(short, long)]
    //         full: bool,
    //
    //         /// Do not rename folders with feed names and plugin instance titles
    //         #[clap(short, long)]
    //         raw: bool,
    //
    //         /// (Swift) data path
    //         #[clap(default_value = "")]
    //         path: String,
    //     },
    //
    //     //
    //     // /// Search for plugins and pipelines
    //     // Search {},
    //     //
    //     /// Get the parameters of a ChRIS plugin.
    //     PluginHelp {
    //         /// Name of a ChRIS plugin
    //         #[clap(required = true)]
    //         plugin_name: PluginName,
    //     },
    //     //
    //     /// Create a plugin instance by name.
    //     #[command(group(
    //     ArgGroup::new("cpu_request").required(false).args(["cpu", "cpu_limit"]),
    //     ))]
    //     RunLatest {
    //         /// CPU resource request, as number of CPU cores.
    //         #[clap(short = 'j', long, value_name = "N")]
    //         cpu: Option<u16>,
    //
    //         /// CPU resource request.
    //         /// Format is xm where x is an integer in millicores.
    //         #[clap(long)]
    //         cpu_limit: Option<String>,
    //
    //         /// Memory resource request.
    //         /// Format is xMi or xGi where x is an integer.
    //         #[clap(short, long)]
    //         memory_limit: Option<String>,
    //
    //         /// GPU resource request.
    //         /// Number of GPUs to use for plugin instance.
    //         #[clap(short, long)]
    //         gpu_limit: Option<u32>,
    //
    //         /// Number of workers resource request.
    //         /// Number of compute nodes for parallel job.
    //         #[clap(short, long)]
    //         number_of_workers: Option<u32>,
    //
    //         /// Name of compute resource
    //         #[clap(short, long)]
    //         compute_resource_name: Option<ComputeResourceName>,
    //
    //         /// Plugin instance title
    //         #[clap(short, long)]
    //         title: Option<String>,
    //
    //         /// Parent plugin instance ID, which is the source of input files for ds-type plugins
    //         // TODO support accepting union type for convenience
    //         // e.g. feed URL, plugin instance URL, plugin instance title...
    //         #[clap(short, long)]
    //         previous_id: Option<u32>,
    //
    //         /// Name of plugin to run
    //         #[clap(required = true)]
    //         plugin_name: PluginName,
    //
    //         /// Plugin parameters
    //         parameters: Vec<String>,
    //     },
    //
    //     /// Make an authenticated HTTP GET request
    //     #[clap(
    //         long_about = "Make an authenticated HTTP GET request (for debugging and advanced users)
    //
    // The output of this subcommand can be piped into `jq`, e.g.
    //
    //     chrs get https://cube.chrisproject.org/api/v1/ | jq"
    //     )]
    //     Get {
    //         /// CUBE resource URL
    //         url: String,
    //     },
    //
    //     /// Work with file-representation of pipelines
    //     #[clap(subcommand)]
    //     PipelineFile(PipelineFile),
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let args: Cli = Cli::parse();

    match args.command {
        Commands::Login {
            no_keyring,
            password_stdin,
        } => {
            println!("ok, you are going to be logged in.");
            Ok(())
        }
        Commands::Whoami {} => {
            println!("i am me!");
            Ok(())
        }
    }
}
