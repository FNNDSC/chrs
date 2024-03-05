use crate::client::variant::{RoAccess, RwAccess};
use crate::errors::{check, CubeError};
use crate::models::data::{PluginParameter, PluginResponse};
use crate::models::linked::LinkedModel;
use crate::{PluginInstanceRw, Search};
use serde::Serialize;

/// A [ChrisPlugin]. Call [Plugin::create_instance] to "run" this plugin.
pub type Plugin = LinkedModel<PluginResponse, RwAccess>;

/// A [ChrisPlugin]. You cannot create plugin instances of a [PublicPlugin].
pub type PublicPlugin = LinkedModel<PluginResponse, RoAccess>;

impl Plugin {
    /// Create a plugin instance (i.e. "run" a plugin)
    pub async fn create_instance<T: Serialize + ?Sized>(
        &self,
        body: &T,
    ) -> Result<PluginInstanceRw, CubeError> {
        let res = self
            .client
            .post(self.object.instances.as_str())
            .json(body)
            .send()
            .await?;
        let data = check(res).await?.json().await?;
        Ok(LinkedModel {
            client: self.client.clone(),
            object: data,
            phantom: Default::default(),
        })
    }
}

/// ChRIS plugin
pub trait ChrisPlugin {
    /// Get plugin parameters
    fn get_parameters(&self) -> Search<PluginParameter, RoAccess, ()>;
}

impl ChrisPlugin for Plugin {
    fn get_parameters(&self) -> Search<PluginParameter, RoAccess, ()> {
        Search::basic(&self.client, &self.object.parameters)
    }
}

impl ChrisPlugin for PublicPlugin {
    fn get_parameters(&self) -> Search<PluginParameter, RoAccess, ()> {
        Search::basic(&self.client, &self.object.parameters)
    }
}
