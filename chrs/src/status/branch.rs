use chris::{FeedRo, PluginInstanceRo};
use crate::client::RoClient;
use crate::login::UiUrl;
use crate::status::feed::only_print_feed_status;
use color_eyre::eyre::Result;

pub async fn print_branch_status(
    client: &RoClient,
    feed: FeedRo,
    plinst: PluginInstanceRo,
    ui_url: Option<UiUrl>,
) -> Result<()> {
    only_print_feed_status(feed, ui_url).await?;
    // let all_plinst = get_all_plugin_instances(client, plinst.feed_id).await?;
    Ok(())
}
