use clap::{builder::NonEmptyStringValueParser, Parser, Subcommand};

use crate::arg::GivenPluginInstance;
use chris::types::{CubeUrl, Username};

use crate::cd::cd;
use crate::client::Credentials;
use crate::describe::{describe_runnable, DescribeArgs};
use crate::list::{list_feeds, ListFeedArgs};
use crate::login::cmd::{login, logout};
use crate::login::store::Backend;
use crate::login::switch::switch_login;
use crate::login::UiUrl;
use crate::logs::logs;
use crate::ls::{ls, LsArgs};
use crate::search::{search_runnable, SearchArgs};
use crate::status::cmd::status;
use crate::whoami::whoami;

mod arg;
mod cd;
mod client;
mod describe;
mod files;
mod list;
mod login;
mod logs;
mod ls;
mod search;
mod shlex;
mod status;
pub mod unicode;
mod whoami;
mod plugin_clap;
mod run;

#[derive(Parser)]
#[clap(
    version,
    about = "ChRIS Research Integration System -- command line client",
    propagate_version = false,
    disable_help_subcommand = true
)]
struct Cli {
    /// ChRIS backend API URL
    #[clap(long, global = true)]
    cube: Option<CubeUrl>,

    /// ChRIS_ui URL
    #[clap(long, global = true)]
    ui: Option<UiUrl>,

    /// account username
    #[clap(long, global = true)]
    username: Option<Username>,

    /// account password
    #[clap(long, global = true)]
    password: Option<String>,

    /// authorization token
    #[clap(long, global = true)]
    token: Option<String>,

    /// Number of times to retry HTTP requests
    #[clap(long)]
    retries: Option<u32>,

    /// Maximum number of concurrent HTTP requests
    #[clap(short = 'j', long, default_value_t = 4)]
    threads: usize,

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

    /// Remove a user session
    Logout {},
    /// Switch user
    Switch {},
    /// Show login information
    Whoami {},

    /// List files
    Ls(LsArgs),

    /// List feeds
    List(ListFeedArgs),

    /// Search for plugins and pipelines
    Search(SearchArgs),

    /// Change plugin instance context
    Cd {
        /// Plugin instance to switch to.
        ///
        /// The value can be a plugin instance ID or title. For a title,
        /// the title must be unique within the search space. The current
        /// feed will be searched before searching across all feeds.
        plugin_instance: GivenPluginInstance,
    },

    /// Show status of a feed branch
    Status {
        /// Print plugin execshell and selfpath
        #[clap(short, long)]
        execshell: bool,

        /// Feed or plugin instance
        #[clap(value_parser = NonEmptyStringValueParser::new())]
        feed_or_plugin_instance: Option<String>,
    },

    /// Show the logs of a plugin instance
    Logs {
        /// Plugin instance
        #[clap(default_value_t)]
        plugin_instance: GivenPluginInstance,
    },

    /// Describe and get usage of a plugin or pipeline
    Describe(DescribeArgs),


                            // /// Get detailed information about a ChRIS object
                            // ///
                            // /// An object may be a plugin, plugin instance, pipeline, feed, or file.
                            // // Future work: also support PACS files
                            // Describe(String),
                            //
                            // /// Run a plugin or pipeline
                            // // TODO: can also create feed and upload files
                            // Run { },

                            // Future work
                            // /// Set name or title of a feed or plugin instance
                            // Set {},

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
                            //
                            //     //
                            //     // /// Search for plugins and pipelines
                            //     // Search {},
                            //     //
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
    let credentials = Credentials {
        cube_url: args.cube,
        username: args.username,
        password: args.password,
        token: args.token,
        retries: args.retries,
        ui: args.ui,
    };

    match args.command {
        Commands::Login {
            no_keyring,
            password_stdin,
        } => {
            let backend = if no_keyring {
                Backend::ClearText
            } else {
                Backend::Keyring
            };
            login(credentials, backend, password_stdin).await
        }
        Commands::Switch {} => switch_login(credentials),
        Commands::Whoami {} => whoami(credentials),
        Commands::Logout {} => logout(credentials),

        Commands::Ls(args) => ls(credentials, args).await,
        Commands::Cd { plugin_instance } => cd(credentials, plugin_instance).await,
        Commands::Status {
            feed_or_plugin_instance,
            execshell,
        } => status(credentials, feed_or_plugin_instance, execshell).await,
        Commands::Logs { plugin_instance } => logs(credentials, plugin_instance).await,
        Commands::List(args) => list_feeds(credentials, args).await,
        Commands::Search(args) => search_runnable(credentials, args).await,
        Commands::Describe(args) => describe_runnable(credentials, args).await,
    }
}
