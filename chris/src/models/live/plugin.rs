use serde::Serialize;

use crate::client::access::{RoAccess, RwAccess};
use crate::errors::CubeError;
use crate::models::data::{PluginParameter, PluginResponse};
use crate::models::linked::LinkedModel;
use crate::search::SearchBuilder;
use crate::{Access, PluginInstanceRw};

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
        self.post(&self.object.instances, body).await
    }
}

impl<A: Access> LinkedModel<PluginResponse, A> {
    pub fn parameters(&self) -> SearchBuilder<PluginParameter, A> {
        self.get_collection(&self.object.parameters)
    }
}
