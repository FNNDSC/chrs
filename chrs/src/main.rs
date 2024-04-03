use clap::{Parser, Subcommand};

use chris::types::{CubeUrl, Username};

use crate::arg::GivenDataNode;
use crate::cd::cd;
use crate::credentials::Credentials;
use crate::describe::{describe_runnable, DescribeArgs};
use crate::download::{download, DownloadArgs};
use crate::list::{list_feeds, ListFeedArgs};
use crate::login::cmd::{login, logout};
use crate::login::store::Backend;
use crate::login::switch::switch_login;
use crate::login::UiUrl;
use crate::logs::logs;
use crate::ls::{ls, LsArgs};
use crate::run::{run_command, RunArgs};
use crate::search::{search_runnable, SearchArgs};
use crate::status::cmd::status;
use crate::upload::{upload, UploadArgs};
use crate::whoami::whoami;

mod arg;
mod cd;
mod credentials;
mod describe;
mod download;
mod error_messages;
mod file_transfer;
mod files;
mod list;
mod login;
mod logs;
mod ls;
mod plugin_clap;
mod run;
mod search;
mod shlex;
mod status;
pub mod unicode;
mod upload;
mod whoami;

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
        plugin_instance: GivenDataNode,
    },

    /// Show status of a feed branch
    Status {
        /// Print plugin execshell and selfpath
        #[clap(short, long)]
        execshell: bool,

        /// Feed or plugin instance
        feed_or_plugin_instance: Option<GivenDataNode>,
    },

    /// Show the logs of a plugin instance
    Logs {
        /// Plugin instance
        plugin_instance: Option<GivenDataNode>,
    },

    /// Describe and get usage of a plugin or pipeline
    Describe(DescribeArgs),

    /// Run a plugin or pipeline
    Run(RunArgs),

    /// Upload files to ChRIS
    Upload(UploadArgs),

    /// Download files from ChRIS
    Download(DownloadArgs),
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
}

#[tokio::main]
async fn main() -> color_eyre::eyre::Result<()> {
    #[cfg(debug_assertions)]
    color_eyre::install()?;
    #[cfg(not(debug_assertions))]
    color_eyre::config::HookBuilder::default()
        // .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
        // .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .display_location_section(false)
        .install()?;

    let args: Cli = Cli::parse();
    let credentials = Credentials {
        cube_url: args.cube,
        username: args.username,
        password: args.password,
        token: args.token,
        retries: args.retries,
        ui: args.ui,
        config_path: None,
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
        Commands::Run(args) => run_command(credentials, args).await,
        Commands::Download(args) => download(credentials, args).await,
        Commands::Upload(args) => upload(credentials, args).await,
    }
}
