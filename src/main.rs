mod upload;
mod chris;

use std::path::PathBuf;
use std::process;

use clap::{AppSettings, Parser, Subcommand};
use crate::chris::ChrisClient;
use crate::upload::upload;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::DisableHelpSubcommand))]
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
    command: Commands
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
        path: String
    },
}

fn main() {
    let args = Cli::parse();
    let client = ChrisClient::new(&args.address, &args.username, &args.password);

    match &args.command {
        Commands::Upload { files , path} => {
            match upload(&client, files, path) {
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1)
                }
                _ => {}
            }
        }
    }
}
