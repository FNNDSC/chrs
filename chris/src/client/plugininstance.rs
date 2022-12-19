use crate::client::feed::ShallowFeed;
use crate::models::PluginInstanceResponse;
use reqwest::Client;

pub struct PluginInstance {
    client: Client,
    pub plugin_instance: PluginInstanceResponse,
}

impl PluginInstance {
    pub(crate) fn new(client: Client, res: PluginInstanceResponse) -> Self {
        PluginInstance {
            client,
            plugin_instance: res,
        }
    }

    pub fn feed(&self) -> ShallowFeed {
        ShallowFeed::new(self.client.clone(), self.plugin_instance.feed.clone())
    }
}
