use color_eyre::eyre;
use chris::{BaseChrisClient, ChrisClient, PluginInstanceResponse};
use chris::types::PluginInstanceId;
use crate::get_client::{Client};
use color_eyre::eyre::{bail, OptionExt, Result};
use color_eyre::owo_colors::OwoColorize;
use futures::TryStreamExt;
use itertools::Itertools;

/// A user-provided string which is supposed to refer to an existing plugin instance.
#[derive(Debug, PartialEq, Clone)]
pub enum GivenPluginInstance {
    Title(String),
    Id(PluginInstanceId),
    Parent(u32),
}

impl From<String> for GivenPluginInstance {
    fn from(value: String) -> Self {
        if let Some(count) = parse_parent_dirs(&value) {
            return GivenPluginInstance::Parent(count);
        }
        if let Some(id) = parse_id_from_url(&value) {
            return GivenPluginInstance::Id(id);
        }
        let right_part = if let Some((left, right)) = value.split_once('/') {
            if left == "pi" || left == "plugininstance" {
                right
            } else {
                &value
            }
        } else {
            value.as_str()
        };
        right_part
            .parse::<u32>()
            .map(PluginInstanceId)
            .map(GivenPluginInstance::Id)
            .unwrap_or_else(|_e| GivenPluginInstance::Title(right_part.to_string()))
    }
}

fn parse_parent_dirs(value: &str) -> Option<u32> {
    let mut count = 0;
    for part in value.split('/') {
        if part.is_empty() {
            return not_zero(count);
        }
        if part != ".." {
            return None;
        }
        count += 1;
    }
    not_zero(count)
}

fn parse_id_from_url(url: &str) -> Option<PluginInstanceId> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }
    url.split_once("/api/v1/plugins/instances/")
        .map(|(_, right)| right)
        .and_then(|s| s.strip_suffix('/'))
        .and_then(|s| s.parse().ok())
        .map(PluginInstanceId)
}

fn not_zero(value: u32) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

impl GivenPluginInstance {
    pub async fn get_using(self, client: &Client, old: Option<PluginInstanceId>) -> Result<PluginInstanceResponse> {
        match self {
            Self::Id(id) => client.get_plugin_instance(id).await.map_err(eyre::Error::new),
            Self::Title(title) => get_by_title(client, title, old).await,
            Self::Parent(count) => get_parents(client, count, old).await
        }
    }
}


async fn get_by_title(
    client: &Client,
    name: String,
    old: Option<PluginInstanceId>,
) -> Result<PluginInstanceResponse> {
    if let Client::LoggedIn(chris) = client {
        search_title(chris, name, old).await
    } else {
        bail!("Cannot search for plugin instances without a user account. Please tell Jorge to fix https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530")
    }
}

async fn search_title(
    chris: &ChrisClient,
    title: String,
    old: Option<PluginInstanceId>,
) -> Result<PluginInstanceResponse> {
    if let Some(old) = old {
        if let Some(res) = search_title_within_feed(chris, title.clone(), old).await? {
            return Ok(res);
        }
    }
    search_title_any_feed(chris, title).await
}

async fn search_title_within_feed(
    chris: &ChrisClient,
    title: String,
    old: PluginInstanceId,
) -> Result<Option<PluginInstanceResponse>> {
    let old = chris.get_plugin_instance(old).await?;
    let query = chris
        .plugin_instances()
        .feed_id(old.object.feed_id)
        .title(title)
        .limit(10);
    let items: Vec<_> = query.search().stream().try_collect().await?;
    let ids: Vec<_> = items.iter().map(|p| p.id).collect();
    if ids.len() > 1 {
        bail!(
            "Multiple plugin instances found. Please specify: {}",
            ids.iter().map(|i| i.to_string()).join(", ")
        );
    }
    Ok(items.into_iter().next())
}

async fn search_title_any_feed(chris: &ChrisClient, title: String) -> Result<PluginInstanceResponse> {
    let query = chris.plugin_instances().title(title);
    let items: Vec<_> = query.search().stream().try_collect().await?;
    let ids: Vec<_> = items.iter().map(|p| p.id).collect();
    if ids.len() > 1 {
        bail!(
            "Multiple plugin instances found. Please specify: {}",
            ids.iter().map(|i| i.to_string()).join(", ")
        );
    }
    items.into_iter()
        .next()
        .ok_or_eyre("Plugin instance not found")
}

async fn get_parents(
    client: &Client,
    parents: u32,
    old: Option<PluginInstanceId>,
) -> Result<PluginInstanceResponse> {
    let old_id = old.ok_or_eyre("No current plugin instance context, cannot get previous.")?;
    let mut current = client.get_plugin_instance(old_id).await?;
    for i in 0..parents {
        if let Some(previous_id) = current.previous_id {
            current = client.get_plugin_instance(previous_id).await?;

        } else {
            eprintln!(
                "warning: wanted to go up {} previous plugin instances, but only found up to {}",
                parents.bright_cyan(),
                i.bold()
            );
            return Ok(current);
        }
    }
    Ok(current)
}

#[cfg(test)]
mod tests {

    use super::*;
    use rstest::*;

    #[rstest]
    #[case("hello", GivenPluginInstance::Title("hello".to_string()))]
    #[case("pi/hello", GivenPluginInstance::Title("hello".to_string()))]
    #[case("plugininstance/hello", GivenPluginInstance::Title("hello".to_string()))]
    #[case("42", GivenPluginInstance::Id(PluginInstanceId(42)))]
    #[case("pi/42", GivenPluginInstance::Id(PluginInstanceId(42)))]
    #[case("plugininstance/42", GivenPluginInstance::Id(PluginInstanceId(42)))]
    #[case(
        "https://example.org/api/v1/plugins/instances/42/",
        GivenPluginInstance::Id(PluginInstanceId(42))
    )]
    #[case("..", GivenPluginInstance::Parent(1))]
    #[case("../", GivenPluginInstance::Parent(1))]
    #[case("../..", GivenPluginInstance::Parent(2))]
    #[case("../../", GivenPluginInstance::Parent(2))]
    fn test_given_plugin_instance(#[case] given: &str, #[case] expected: GivenPluginInstance) {
        let actual: GivenPluginInstance = given.to_string().into();
        assert_eq!(actual, expected)
    }
}
