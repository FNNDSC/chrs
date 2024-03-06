use crate::arg::GivenPluginInstance;
use crate::client::{Client, Credentials, RoClient};
use crate::login::UiUrl;
use crate::unicode;
use chris::types::{FeedId, PluginInstanceId};
use chris::{FeedResponse, FeedRo, PluginInstanceResponse};
use color_eyre::eyre::{Error, OptionExt, Result};
use color_eyre::owo_colors::OwoColorize;
use std::fmt::Display;

pub async fn status(
    credentials: Credentials,
    feed_or_plugin_instance: Option<String>,
) -> Result<()> {
    let (client, current_plinst, ui) = credentials
        .get_client(feed_or_plugin_instance.as_ref().as_slice())
        .await?;
    let fopi = feed_or_plugin_instance
        .or_else(|| {
            current_plinst
                .as_ref()
                .map(|i| format!("plugininstance/{}", i.0))
        })
        .ok_or_eyre("missing operand")?;
    let given = GivenFeedOrPluginInstance::from(fopi);
    let (feed, plinst) = given.resolve_using(&client, current_plinst).await?;
    print_status(client.into_ro(), feed, plinst, ui).await
}

enum GivenFeedOrPluginInstance {
    FeedId(FeedId),
    FeedName(String),
    PluginInstance(GivenPluginInstance),
    Ambiguous(String),
}

impl From<String> for GivenFeedOrPluginInstance {
    fn from(value: String) -> Self {
        if let Some(id) = parse_feed_id_from_url(&value) {
            return Self::FeedId(id);
        }
        value
            .split_once('/')
            .and_then(|(left, right)| {
                if left == "f" || left == "feed" {
                    Some(
                        right
                            .parse::<u32>()
                            .map(FeedId)
                            .map(Self::FeedId)
                            .unwrap_or(Self::FeedName(right.to_string())),
                    )
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let plinst = GivenPluginInstance::from(value);
                if let GivenPluginInstance::Title(ambiguous) = plinst {
                    Self::Ambiguous(ambiguous)
                } else {
                    Self::PluginInstance(plinst)
                }
            })
    }
}

impl GivenFeedOrPluginInstance {
    async fn resolve_using(
        self,
        client: &Client,
        old: Option<PluginInstanceId>,
    ) -> Result<(Option<FeedRo>, Option<PluginInstanceResponse>)> {
        match self {
            GivenFeedOrPluginInstance::FeedId(id) => client
                .get_feed(id)
                .await
                .map(|f| (Some(f), None))
                .map_err(Error::new),
            GivenFeedOrPluginInstance::FeedName(name) => client
                .get_feed_by_name(&name)
                .await
                .map(|f| (Some(f), None)),
            GivenFeedOrPluginInstance::PluginInstance(p) => get_plinst_and_feed(client, p, old)
                .await
                .map(|(f, p)| (Some(f), Some(p))),
            GivenFeedOrPluginInstance::Ambiguous(_) => Err(Error::msg(
                "Operand is ambiguous, resolution not implemented",
            )),
        }
    }
}

async fn get_plinst_and_feed(
    client: &Client,
    p: GivenPluginInstance,
    old: Option<PluginInstanceId>,
) -> Result<(FeedRo, PluginInstanceResponse)> {
    let plinst = p.get_using(client, old).await?;
    let feed = client.get_feed(plinst.feed_id).await?;
    Ok((feed, plinst))
}

fn parse_feed_id_from_url(url: &str) -> Option<FeedId> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }
    url.split_once("/api/v1/")
        .map(|(_, right)| right)
        .and_then(|s| s.strip_suffix('/'))
        .and_then(|s| s.parse().ok())
        .map(FeedId)
}

async fn print_status(
    client: RoClient,
    feed: Option<FeedRo>,
    plinst: Option<PluginInstanceResponse>,
    ui_url: Option<UiUrl>,
) -> Result<()> {
    if let Some(plugin_instance) = plinst {
        if let Some(feed) = feed {
            print_branch_status(&client, feed, plugin_instance, ui_url).await
        } else {
            let feed = client.get_feed(plugin_instance.feed_id).await?;
            print_branch_status(&client, feed, plugin_instance, ui_url).await
        }
    } else if let Some(feed) = feed {
        only_print_feed_status(feed, ui_url).await
    } else {
        Ok(())
    }
}

async fn only_print_feed_status(feed: FeedRo, ui_url: Option<UiUrl>) -> Result<()> {
    let symbol = feed_symbol_for(&feed.object);
    let name = if feed.object.name.is_empty() {
        ""
    } else {
        feed.object.name.as_str()
    };

    let styled_name = if feed.object.has_errored_job() {
        name.bold().bright_red().to_string()
    } else {
        name.bold().bright_green().to_string()
    };

    println!("{} {}", symbol, styled_name);
    if let Some(ui) = ui_url {
        println!("  {}", ui.feed_url_of(&feed.object).underline())
    }
    let bar = "  |".dimmed();
    println!("{}", &bar);
    println!(
        "{}   {}",
        &bar,
        format!("{}: {}", " created", feed.object.creation_date.italic()).dimmed()
    );
    println!(
        "{}   {}",
        &bar,
        format!("{}: {}", "modified", feed.object.modification_date.italic()).dimmed()
    );
    println!("{}", &bar);
    println!(
        "{}   {}",
        &bar,
        format!(
            "finished: {}  pending: {}  running: {}  errors: {}",
            feed.object.finished_jobs,
            feed.object.pending_jobs(),
            feed.object.running_jobs(),
            feed.object.errored_jobs
        )
        .dimmed()
    );

    let note = feed.get_note().await?;
    if !note.is_empty() {
        println!("{}", &bar);
        println!("{} {}", &bar, note.object.content);  // TODO split lines to fit screen
    }

    Ok(())
}

fn feed_symbol_for(feed: &FeedResponse) -> impl Display {
    if feed.has_errored_job() {
        unicode::BLACK_DOWN_POINTING_TRIANGLE
            .bold()
            .red()
            .to_string()
    } else if feed.has_unfinished_jobs() {
        unicode::BLACK_UP_POINTING_TRIANGLE
            .bold()
            .yellow()
            .to_string()
    } else {
        unicode::BLACK_UP_POINTING_TRIANGLE
            .bold()
            .green()
            .to_string()
    }
}

async fn print_branch_status(
    client: &RoClient,
    feed: FeedRo,
    plinst: PluginInstanceResponse,
    ui_url: Option<UiUrl>,
) -> Result<()> {
    only_print_feed_status(feed, ui_url).await?;
    println!("PRINTING THE PLUGIN INSTANCE BRANCH IS NOT YET IMPLEMENTED");
    Ok(())
}
