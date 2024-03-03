use crate::errors::{check, CubeError};
use crate::models::data::{AuthedPluginResponse, PluginInstanceResponse, PluginParameter};
use crate::models::linked::LinkedModel;
use crate::models::AnonPluginResponse;
use crate::Search;
use serde::Serialize;

/// ChRIS plugin
pub trait ChrisPlugin {
    /// Get plugin parameters
    fn get_parameters(&self) -> Search<PluginParameter, ()>;
}

/// A [ChrisPlugin]. Call [Plugin::create_instance] to "run" this plugin.
pub type Plugin = LinkedModel<AuthedPluginResponse>;

impl Plugin {
    /// Create a plugin instance (i.e. "run" a plugin)
    pub async fn create_instance<T: Serialize + ?Sized>(
        &self,
        body: &T,
    ) -> Result<PluginInstanceResponse, CubeError> {
        let res = self
            .client
            .post(self.object.instances.as_str())
            .json(body)
            .send()
            .await?;
        let data = check(res).await?.json().await?;
        Ok(data)
    }
}

impl ChrisPlugin for Plugin {
    fn get_parameters(&self) -> Search<PluginParameter, ()> {
        Search::basic(&self.client, &self.object.parameters)
    }
}

/// A [ChrisPlugin]. You cannot create plugin instances of a [PublicPlugin].
pub type PublicPlugin = LinkedModel<AnonPluginResponse>;

impl ChrisPlugin for PublicPlugin {
    fn get_parameters(&self) -> Search<PluginParameter, ()> {
        Search::basic(&self.client, &self.object.parameters)
    }
}
