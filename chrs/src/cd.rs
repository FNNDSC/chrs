use crate::arg::GivenPluginInstance;
use crate::get_client::{Client, Credentials, RoClient};
use crate::login::state::ChrsSessions;
use chris::types::PluginInstanceId;
use chris::{BaseChrisClient, ChrisClient};
use color_eyre::eyre::{bail, OptionExt, Result};
use color_eyre::owo_colors::OwoColorize;
use futures::TryStreamExt;
use itertools::Itertools;

pub async fn cd(credentials: Credentials, given_plinst: String) -> Result<()> {
    let (client, old_plinst) = credentials.clone().get_client([&given_plinst]).await?;
    let given_plinst = GivenPluginInstance::from(given_plinst);
    let cube_url = client.url().clone();
    let username = client.username();
    let plinst = match given_plinst {
        GivenPluginInstance::Title(name) => get_by_title(&client, name, old_plinst).await,
        GivenPluginInstance::Id(id) => get_by_id(client.into_ro(), id).await,
        GivenPluginInstance::Parent(count) => get_parent(client.into_ro(), old_plinst, count).await,
    }?;
    let mut sessions = ChrsSessions::load()?;
    sessions.set_plugin_instance(&cube_url, &username, plinst);
    sessions.save()
}

async fn get_by_title(
    client: &Client,
    name: String,
    old: Option<PluginInstanceId>,
) -> Result<PluginInstanceId> {
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
) -> Result<PluginInstanceId> {
    if let Some(old) = old {
        if let Some(id) = search_title_within_feed(chris, title.clone(), old).await? {
            return Ok(id);
        }
    }
    search_title_any_feed(chris, title).await
}

async fn search_title_within_feed(
    chris: &ChrisClient,
    title: String,
    old: PluginInstanceId,
) -> Result<Option<PluginInstanceId>> {
    let old = chris.get_plugin_instance(old).await?;
    let query = chris
        .plugin_instances()
        .feed_id(old.object.feed_id)
        .title(title)
        .limit(10);
    let items: Vec<_> = query.search().stream().try_collect().await?;
    let ids: Vec<_> = items.into_iter().map(|p| p.id).collect();
    if ids.len() > 1 {
        bail!(
            "Multiple plugin instances found. Please specify: {}",
            ids.iter().map(|i| i.to_string()).join(", ")
        );
    }
    Ok(ids.into_iter().next())
}

async fn search_title_any_feed(chris: &ChrisClient, title: String) -> Result<PluginInstanceId> {
    let query = chris.plugin_instances().title(title);
    let items: Vec<_> = query.search().stream().try_collect().await?;
    let ids: Vec<_> = items.into_iter().map(|p| p.id).collect();
    if ids.len() > 1 {
        bail!(
            "Multiple plugin instances found. Please specify: {}",
            ids.iter().map(|i| i.to_string()).join(", ")
        );
    }
    ids.into_iter()
        .next()
        .ok_or_eyre("Plugin instance not found")
}

async fn get_by_id(client: RoClient, id: PluginInstanceId) -> Result<PluginInstanceId> {
    let plinst = client.get_plugin_instance(id).await?.object;
    Ok(plinst.id)
}

async fn get_parent(
    client: RoClient,
    old: Option<PluginInstanceId>,
    parents: u32,
) -> Result<PluginInstanceId> {
    let mut id = old.ok_or_eyre("No current plugin instance context, cannot cd into parent.")?;
    for i in 0..parents {
        if let Some(previous_id) = client.get_plugin_instance(id).await?.object.previous_id {
            id = previous_id;
        } else {
            eprintln!(
                "warning: wanted to go up {} previous plugin instances, but only found up to {}",
                parents.bright_cyan(),
                i.bold()
            );
            return Ok(id);
        }
    }
    Ok(id)
}
