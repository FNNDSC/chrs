use crate::client::feed::ShallowFeed;
use crate::models::PluginInstanceCreatedResponse;
use reqwest::Client;

pub struct PluginInstance {
    client: Client,
    pub plugin_instance: PluginInstanceCreatedResponse,
}

impl PluginInstance {
    pub(crate) fn new(client: Client, res: PluginInstanceCreatedResponse) -> Self {
        PluginInstance {
            client,
            plugin_instance: res,
        }
    }

    pub fn get_feed(&self) -> ShallowFeed {
        ShallowFeed::new(self.client.clone(), self.plugin_instance.feed.clone())
    }
}
