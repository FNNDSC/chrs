use color_eyre::eyre;
use color_eyre::eyre::{bail, eyre, Error, OptionExt};
use color_eyre::owo_colors::OwoColorize;
use futures::TryStreamExt;
use itertools::Itertools;

use chris::types::{FeedId, PluginInstanceId};
use chris::{
    Access, BaseChrisClient, ChrisClient, EitherClient, Feed, FeedRo, PluginInstance,
    PluginInstanceRo, PluginInstanceRw, RoAccess,
};

use crate::arg::GivenPluginInstanceOrPath;
use crate::error_messages::CANNOT_ANONYMOUSLY_SEARCH;

/// A user-provided string resolved as either a feed, plugin instance, or _ChRIS_ filesystem path.
#[derive(Debug, Clone)]
pub enum GivenDataNode {
    FeedId { id: FeedId, original: String },
    FeedName(String),
    PluginInstanceOrPath(GivenPluginInstanceOrPath),
    Ambiguous(String),
}

/// Union type of [Feed] and [PluginInstance]
pub enum FeedOrPluginInstance<A: Access> {
    Feed(Feed<A>),
    PluginInstance(PluginInstance<A>),
}

impl From<String> for GivenDataNode {
    fn from(value: String) -> Self {
        if let Some(id) = parse_feed_id_from_url(&value) {
            GivenDataNode::FeedId {
                id,
                original: value,
            }
        } else {
            value
                .split_once('/')
                .and_then(parse_split_qualified_feed)
                .unwrap_or_else(|| differentiate_plinst_or_ambiguous(value))
        }
    }
}

impl From<PluginInstanceId> for GivenDataNode {
    fn from(value: PluginInstanceId) -> Self {
        let orig = format!("plugininstance/{}", value.0);
        GivenDataNode::PluginInstanceOrPath(GivenPluginInstanceOrPath::Id(value, orig))
    }
}

/// Handle parsing of a user-provided string which was split once on '/'.
///
/// If the left part is "f" or "feed", it is identified as a feed.
fn parse_split_qualified_feed((left, right): (&str, &str)) -> Option<GivenDataNode> {
    if left == "f" || left == "feed" {
        Some(
            right
                .parse::<u32>()
                .map(FeedId)
                .map(|id| GivenDataNode::FeedId {
                    id,
                    original: right.to_string(),
                })
                .unwrap_or(GivenDataNode::FeedName(right.to_string())),
        )
    } else {
        None
    }
}

/// Attempt to parse the value as a [GivenPluginInstanceOrPath], but resolve as
/// [GivenDataNode::Ambiguous] if value could be either a feed name or plugin instance title.
fn differentiate_plinst_or_ambiguous(value: String) -> GivenDataNode {
    let plinst = GivenPluginInstanceOrPath::from(value);
    if let GivenPluginInstanceOrPath::Title(ambiguous) = plinst {
        GivenDataNode::Ambiguous(ambiguous)
    } else {
        GivenDataNode::PluginInstanceOrPath(plinst)
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

impl GivenDataNode {
    pub fn as_arg_str(&self) -> &str {
        match self {
            GivenDataNode::FeedId { original, .. } => original.as_str(),
            GivenDataNode::FeedName(name) => name.as_str(),
            GivenDataNode::PluginInstanceOrPath(p) => p.as_arg_str(),
            GivenDataNode::Ambiguous(s) => s.as_str(),
        }
    }

    /// Get the CUBE object.
    pub async fn into_or(
        self,
        client: &EitherClient,
        old: Option<PluginInstanceId>,
    ) -> eyre::Result<FeedOrPluginInstance<RoAccess>> {
        match self {
            GivenDataNode::FeedId { id, .. } => client
                .get_feed(id)
                .await
                .map(FeedOrPluginInstance::Feed)
                .map_err(Error::new),
            GivenDataNode::FeedName(name) => get_feedro_by_name(client, &name)
                .await
                .map(FeedOrPluginInstance::Feed),
            GivenDataNode::PluginInstanceOrPath(p) => p.get_using_either(client, old).await
                .map(FeedOrPluginInstance::PluginInstance),
            GivenDataNode::Ambiguous(_) => Err(Error::msg(
                "Operand is ambiguous, cannot differentiate between feed name or plugin instance title",
            )),
        }
    }

    /// Get the CUBE object interpreted as a plugin instance.
    ///
    /// - Plugin instances are returned as plugin instances (duh)
    /// - Feeds will be resolved to their most recent plugin instance
    /// - Paths will be resolved to a plugin instance by their output, if possible.
    /// - Ambiguous value assumed to be a [GivenPluginInstanceOrPath]
    pub async fn into_plinst_rw(
        self,
        client: &ChrisClient,
        old: Option<PluginInstanceId>,
    ) -> eyre::Result<PluginInstanceRw> {
        match self {
            GivenDataNode::FeedId { id, .. } => get_plinst_of_feed(client, id).await,
            GivenDataNode::FeedName(name) => {
                let feed_id = get_feedid_by_name(client, name).await?;
                get_plinst_of_feed(client, feed_id).await
            }
            GivenDataNode::PluginInstanceOrPath(given) => given.get_using_rw(client, old).await,
            GivenDataNode::Ambiguous(value) => {
                GivenPluginInstanceOrPath::from(value)
                    .get_using_rw(client, old)
                    .await
            }
        }
    }

    /// Get the CUBE object interpreted as a plugin instance.
    pub async fn into_plinst_either(
        self,
        client: &EitherClient,
        old: Option<PluginInstanceId>,
    ) -> eyre::Result<PluginInstanceRo> {
        if let Some(logged_in) = client.logged_in_ref() {
            match self {
                GivenDataNode::FeedId { id, .. } => {
                    return get_plinst_of_feed(logged_in, id).await.map(|p| p.into());
                }
                GivenDataNode::FeedName(name) => {
                    let feed_id = get_feedid_by_name(logged_in, name).await?;
                    return get_plinst_of_feed(logged_in, feed_id)
                        .await
                        .map(|p| p.into());
                }
                _ => (),
            }
        };
        match self {
            GivenDataNode::FeedId { .. } => Err(eyre!(CANNOT_ANONYMOUSLY_SEARCH)),
            GivenDataNode::FeedName(_) => Err(eyre!(CANNOT_ANONYMOUSLY_SEARCH)),
            GivenDataNode::PluginInstanceOrPath(given) => given.get_using_either(client, old).await,
            GivenDataNode::Ambiguous(given) => {
                GivenPluginInstanceOrPath::from(given)
                    .get_using_either(client, old)
                    .await
            }
        }
    }

    /// Interpret this as a path.
    ///
    /// - Absolute paths are returns as themselves (duh)
    /// - Relative paths will be resolved relative to `old.output_dir`
    /// - Plugin instances will resolve to the parent of their output path (one level above `data/`)
    /// - Feeds will resolve to that of the output path of its most recent plugin instance
    pub async fn into_path(
        self,
        client: &EitherClient,
        old: Option<PluginInstanceId>,
    ) -> eyre::Result<String> {
        if let Some(logged_in) = client.logged_in_ref() {
            match self {
                GivenDataNode::FeedId { id, .. } => {
                    return get_plinst_of_feed(logged_in, id).await.map(plinst_path);
                }
                GivenDataNode::FeedName(name) => {
                    let feed_id = get_feedid_by_name(logged_in, name).await?;
                    return get_plinst_of_feed(logged_in, feed_id)
                        .await
                        .map(plinst_path);
                }
                _ => (),
            }
        }
        match self {
            GivenDataNode::FeedId { .. } => Err(eyre!(CANNOT_ANONYMOUSLY_SEARCH)),
            GivenDataNode::FeedName(_) => Err(eyre!(CANNOT_ANONYMOUSLY_SEARCH)),
            GivenDataNode::PluginInstanceOrPath(given) => given.into_path(client, old).await,
            GivenDataNode::Ambiguous(given) => {
                GivenPluginInstanceOrPath::from(given)
                    .into_path(client, old)
                    .await
            }
        }
    }
}

fn plinst_path<A: Access>(p: PluginInstance<A>) -> String {
    p.object
        .output_path
        .strip_suffix("/data")
        .map(|p| p.to_string())
        .unwrap_or(p.object.output_path)
}

/// Get the first plugin instance of a feed returned from CUBE's API,
/// which we assume to be the most recently created plugin instance
/// of that feed.
async fn get_plinst_of_feed(
    client: &ChrisClient,
    feed_id: FeedId,
) -> eyre::Result<PluginInstanceRw> {
    client
        .plugin_instances()
        .feed_id(feed_id)
        .page_limit(1)
        .max_items(1)
        .search()
        .get_first()
        .await?
        .ok_or_else(|| {
            eyre!(
                "feed/{} does not contain plugin instances. This is a CUBE bug.",
                feed_id.0
            )
        })
}

async fn get_feedid_by_name(client: &ChrisClient, name: String) -> eyre::Result<FeedId> {
    let query = client.feeds().name_exact(name).page_limit(2).max_items(2);
    let search = query.search();
    let items: Vec<_> = search.stream().map_ok(|f| f.id).try_collect().await?;
    if items.len() > 1 {
        bail!("Multiple feeds found, please be more specific.\nHint: run `{}` and specify feed by feed/{}", "chrs list".bold(), "ID".bold().green())
    }
    items.into_iter().next().ok_or_eyre("Feed not found")
}

/// Gets a feed by name.
///
/// In the case of anonymous access, it's trivial.
///
/// For authenticated client, it is necessary to search both public feeds and private feeds separately.
/// See https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530
async fn get_feedro_by_name(client: &EitherClient, name: &str) -> color_eyre::Result<FeedRo> {
    let feeds: Vec<_> = match client {
        EitherClient::Anon(c) => {
            c.public_feeds()
                .name(name)
                .page_limit(10)
                .max_items(10)
                .search()
                .stream_connected()
                .try_collect()
                .await
        }
        EitherClient::LoggedIn(c) => {
            let private_feeds: Vec<_> = c
                .feeds()
                .name(name)
                .page_limit(10)
                .max_items(10)
                .search()
                .stream_connected()
                .map_ok(|f| f.into())
                .try_collect()
                .await?;
            if private_feeds.is_empty() {
                c.public_feeds()
                    .name(name)
                    .page_limit(10)
                    .max_items(10)
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
