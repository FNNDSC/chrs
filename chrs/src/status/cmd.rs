use color_eyre::eyre::Result;
use chris::types::PluginInstanceId;
use crate::get_client::Credentials;

pub async fn status(credentials: Credentials, feed_or_plugin_instance: Option<String>) -> Result<()> {
    let (client, current_plinst) = credentials.get_client(feed_or_plugin_instance.as_ref().as_slice()).await?;

    todo!()
}
