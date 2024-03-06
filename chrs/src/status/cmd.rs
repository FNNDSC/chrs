use crate::arg::GivenPluginInstance;
use crate::client::{Client, Credentials, RoClient};
use crate::login::UiUrl;
use chris::types::{FeedId, PluginInstanceId};
use chris::{FeedRo, PluginInstanceRo};
use color_eyre::eyre::{Error, OptionExt, Result};
use crate::status::branch::print_branch_status;
use super::feed::only_print_feed_status;

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
    ) -> Result<(Option<FeedRo>, Option<PluginInstanceRo>)> {
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
) -> Result<(FeedRo, PluginInstanceRo)> {
    let plinst = p.get_using(client, old).await?;
    let feed = plinst.feed().get().await?;
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
    plinst: Option<PluginInstanceRo>,
    ui_url: Option<UiUrl>,
) -> Result<()> {
    if let Some(plugin_instance) = plinst {
        if let Some(feed) = feed {
            print_branch_status(&client, feed, plugin_instance, ui_url).await
        } else {
            let feed = plugin_instance.feed().get().await?;
            print_branch_status(&client, feed, plugin_instance, ui_url).await
        }
    } else if let Some(feed) = feed {
        only_print_feed_status(feed, ui_url).await
    } else {
        Ok(())
    }
}
