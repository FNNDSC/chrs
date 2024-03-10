use color_eyre::eyre::{OptionExt, Result};

use chris::{FeedRo, PluginInstanceRo};

use crate::arg::GivenFeedOrPluginInstance;
use crate::credentials::Credentials;
use crate::login::UiUrl;

use super::feed::only_print_feed_status;
use super::print_branch::print_branch_status;

pub async fn status(
    credentials: Credentials,
    feed_or_plugin_instance: Option<String>,
    show_execshell: bool,
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
    print_status(feed, plinst, ui, show_execshell).await
}

async fn print_status(
    feed: Option<FeedRo>,
    plinst: Option<PluginInstanceRo>,
    ui_url: Option<UiUrl>,
    show_execshell: bool,
) -> Result<()> {
    if let Some(plugin_instance) = plinst {
        if let Some(feed) = feed {
            print_branch_status(feed, plugin_instance, ui_url, show_execshell).await
        } else {
            let feed = plugin_instance.feed().get().await?;
            print_branch_status(feed, plugin_instance, ui_url, show_execshell).await
        }
    } else if let Some(feed) = feed {
        only_print_feed_status(&feed, ui_url).await
    } else {
        Ok(())
    }
}
