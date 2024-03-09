// There is a lot of code duplication in here, but it works for now.

use clap::Parser;
use color_eyre::eyre;
use color_eyre::eyre::{bail, Result};
use color_eyre::owo_colors::OwoColorize;
use futures::{future, future::Ready, TryStreamExt};

use chris::errors::CubeError;
use chris::{Access, BaseChrisClient, ChrisClient, FeedResponse};

use crate::client::{Client, Credentials, NO_ARGS};
use crate::unicode;

#[derive(Parser)]
pub struct ListFeedArgs {
    /// Show only public feeds
    #[clap(long)]
    public: bool,

    /// Show only private feeds
    #[clap(long, conflicts_with = "public")]
    private: bool,

    /// Do not print header
    #[clap(short, long)]
    no_header: bool,

    /// Feed name to filter by
    #[clap(default_value = "")]
    name: String,
}

pub async fn list_feeds(credentials: Credentials, args: ListFeedArgs) -> Result<()> {
    let (client, _, _) = credentials.get_client(NO_ARGS).await?;
    match client {
        Client::Anon(c) => list_feeds_anon(c, args).await,
        Client::LoggedIn(c) => list_feeds_authed(c, args).await,
    }
}

async fn list_feeds_anon<A: Access>(
    client: impl BaseChrisClient<A>,
    args: ListFeedArgs,
) -> Result<()> {
    if args.private {
        bail!("Cannot list private feeds, not logged in.")
    }
    if !args.no_header {
        println!(
            "{:<13} {:<60}",
            "ID".bold().underline(),
            "Name".bold().underline()
        );
    }
    let search_builder = client.public_feeds().name(&args.name);
    search_builder
        .search()
        .stream()
        .try_for_each(print_feed_id_and_name)
        .await
        .map_err(eyre::Error::new)
}

fn print_feed_id_and_name(feed: FeedResponse) -> Ready<std::result::Result<(), CubeError>> {
    println!("feed/{:<8} {}", feed.id.0.bold(), feed.name);
    future::ok(())
}

async fn list_feeds_authed(client: ChrisClient, args: ListFeedArgs) -> Result<()> {
    if args.public {
        list_feeds_anon(client, args).await
    } else if args.private {
        list_feeds_private(client, args).await
    } else {
        list_feeds_public_and_private(client, args).await
    }
}

async fn list_feeds_private(client: ChrisClient, args: ListFeedArgs) -> Result<()> {
    if !args.no_header {
        println!(
            "{:<13} {:<60}",
            "ID".bold().underline(),
            "Name".bold().underline(),
        );
    }
    let private_feeds = client.feeds().name(&args.name);
    private_feeds
        .search()
        .stream()
        .try_for_each(print_feed_id_and_name)
        .await
        .map_err(eyre::Error::new)
}

async fn list_feeds_public_and_private(client: ChrisClient, args: ListFeedArgs) -> Result<()> {
    let public_feeds_builder = client.public_feeds().name(&args.name);
    let public_feeds = public_feeds_builder.search();
    let private_feeds_builder = client.feeds().name(&args.name);
    let private_feeds = private_feeds_builder.search();
    let stream = tokio_stream::StreamExt::merge(public_feeds.stream(), private_feeds.stream());
    if !args.no_header {
        println!(
            "{:<13} {:<60} {}",
            "ID".bold().underline(),
            "Name".bold().underline(),
            "Public?".bold().underline()
        );
    }
    stream
        .try_for_each(print_public_or_private)
        .await
        .map_err(eyre::Error::new)
}

fn print_public_or_private(feed: FeedResponse) -> Ready<std::result::Result<(), CubeError>> {
    let is_public = if feed.public { unicode::CHECK_MARK } else { "" };
    println!(
        "feed/{:<8} {:<60} {}",
        feed.id.0.bold(),
        feed.name,
        is_public.bold().green()
    );
    future::ok(())
}
