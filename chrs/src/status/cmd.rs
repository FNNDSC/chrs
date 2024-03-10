use color_eyre::eyre::{OptionExt, Result};

use chris::{FeedRo, PluginInstanceRo};

use crate::arg::{FeedOrPluginInstance, GivenDataNode};
use crate::credentials::Credentials;
use crate::login::UiUrl;

use super::feed::only_print_feed_status;
use super::print_branch::print_branch_status;

pub async fn status(
    credentials: Credentials,
    given: Option<GivenDataNode>,
    show_execshell: bool,
) -> Result<()> {
    let (client, old, ui) = credentials
        .get_client(given.as_ref().map(|g| g.as_arg_str()).as_slice())
        .await?;
    let given = given
        .or_else(|| old.map(|id| id.into()))
        .ok_or_eyre("missing operand")?;
    let (feed, plinst) = match given.into_or(&client, old).await? {
        FeedOrPluginInstance::Feed(feed) => (Some(feed), None),
        FeedOrPluginInstance::PluginInstance(p) => {
            let feed = p.feed().get().await?;
            (Some(feed), Some(p))
        }
    };
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
