use crate::models::active::feed::ShallowFeed;
use crate::models::PluginInstanceResponse;
use reqwest::Client;
use crate::models::connected::ConnectedModel;
use crate::models::data::PluginInstanceResponse;

pub type PluginInstance = ConnectedModel<PluginInstanceResponse>;

impl PluginInstance {
    pub fn feed(&self) -> ShallowFeed {
        ShallowFeed::new(self.client.clone(), self.plugin_instance.feed.clone())
    }
}
