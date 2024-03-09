use color_eyre::eyre::{bail, Error, OptionExt};
use futures::TryStreamExt;
use itertools::Itertools;

use chris::types::{FeedId, PluginInstanceId};
use chris::{BaseChrisClient, EitherClient, FeedRo, PluginInstanceRo};

use crate::arg::GivenPluginInstance;

#[derive(Debug, Clone)]
pub enum GivenFeedOrPluginInstance {
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

impl GivenFeedOrPluginInstance {
    pub async fn resolve_using(
        self,
        client: &EitherClient,
        old: Option<PluginInstanceId>,
    ) -> color_eyre::Result<(Option<FeedRo>, Option<PluginInstanceRo>)> {
        match self {
            GivenFeedOrPluginInstance::FeedId(id) => client
                .get_feed(id)
                .await
                .map(|f| (Some(f), None))
                .map_err(Error::new),
            GivenFeedOrPluginInstance::FeedName(name) => get_feed_by_name(client, &name)
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

async fn get_feed_by_name(client: &EitherClient, name: &str) -> color_eyre::Result<FeedRo> {
    let feeds: Vec<_> = match client {
        EitherClient::Anon(c) => {
            let query = c.public_feeds().name(name).page_limit(10).max_items(10);
            query.search().stream_connected().try_collect().await
        }
        EitherClient::LoggedIn(c) => {
            // need to get both public feeds and private feeds
            // https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530
            let private_query = c.feeds().name(name).page_limit(10).max_items(10);
            let private_feeds: Vec<_> = private_query
                .search()
                .stream_connected()
                .map_ok(|f| f.into())
                .try_collect()
                .await?;
            if private_feeds.is_empty() {
                let public_feeds_query =
                    c.public_feeds().name(name).page_limit(10).max_items(10);
                public_feeds_query
                    .search()
                    .stream_connected()
                    .try_collect()
                    .await
            } else {
                Ok(private_feeds)
            }
        }
    }?;
    if feeds.len() > 1 {
        bail!(
            "More than one feed found: {}",
            feeds
                .iter()
                .map(|f| format!("feed/{}", f.object.id.0))
                .join(" ")
        )
    }
    feeds.into_iter().next().ok_or_eyre("Feed not found")
}

async fn get_plinst_and_feed(
    client: &EitherClient,
    p: GivenPluginInstance,
    old: Option<PluginInstanceId>,
) -> color_eyre::Result<(FeedRo, PluginInstanceRo)> {
    let plinst = p.get_using_either(client, old).await?;
    let feed = plinst.feed().get().await?;
    Ok((feed, plinst))
}
