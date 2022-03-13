mod chris;
mod config;
mod login;
mod upload;

use std::path::PathBuf;
use std::process;

use crate::chris::ChrisClient;
use crate::upload::upload;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(
    author, version, about, long_about = None,
    propagate_version = true, disable_help_subcommand = true
)]
struct Cli {
    /// CUBE address
    #[clap(short, long)]
    address: String,

    /// account username
    #[clap(long)]
    username: String,

    /// account password
    #[clap(long)]
    password: String,

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
}

fn main() {
    let args: Cli = Cli::parse();
    let client = ChrisClient::new(&args.address, &args.username, &args.password);

    match &args.command {
        Commands::Upload { files, path } => {
            if let Err(e) = upload(&client, files, path) {
                eprintln!("{}", e);
                process::exit(1)
            }
        }
    }
}
