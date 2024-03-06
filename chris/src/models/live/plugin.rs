use crate::client::access::{RoAccess, RwAccess};
use crate::errors::{check, CubeError};
use crate::models::data::{PluginParameter, PluginResponse};
use crate::models::linked::LinkedModel;
use crate::search::Search;
use crate::{Access, PluginInstanceRw};
use serde::Serialize;

/// A ChRIS plugin. Call [Plugin::create_instance] to "run" this plugin.
pub type Plugin = LinkedModel<PluginResponse, RwAccess>;

/// A publicly accessed plugin. You cannot create plugin instances of a [PublicPlugin].
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

impl<A: Access> LinkedModel<PluginResponse, A> {
    pub fn get_parameters(
        &self,
        max_items: Option<usize>,
    ) -> Search<PluginParameter, RoAccess, ()> {
        Search::collection(&self.client, &self.object.parameters, (), max_items)
    }
}
