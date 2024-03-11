use serde::Serialize;

use crate::client::access::{RoAccess, RwAccess};
use crate::errors::CubeError;
use crate::models::data::{PluginParameter, PluginResponse};
use crate::models::linked::LinkedModel;
use crate::search::Search;
use crate::{Access, ComputeResourceResponse, PluginInstanceRw};

/// A ChRIS plugin.
pub type Plugin<A> = LinkedModel<PluginResponse, A>;

/// A ChRIS plugin. Call [PluginRw::create_instance] to "run" this plugin.
pub type PluginRw = LinkedModel<PluginResponse, RwAccess>;

/// A publicly accessed plugin. You cannot create plugin instances of a [PluginRo].
pub type PluginRo = LinkedModel<PluginResponse, RoAccess>;

impl PluginRw {
    /// Get compute resource this plugin can run on.
    ///
    /// Note: only allowed to call this if logged in.
    /// See https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/540
    pub fn compute_resources(&self) -> Search<ComputeResourceResponse, RwAccess> {
        self.get_collection(&self.object.compute_resources)
    }

    /// Create a plugin instance (i.e. "run" a plugin)
    pub async fn create_instance<T: Serialize + ?Sized>(
        &self,
        body: &T,
    ) -> Result<PluginInstanceRw, CubeError> {
        self.post(&self.object.instances, body).await
    }
}

impl<A: Access> Plugin<A> {
    pub fn parameters(&self) -> Search<PluginParameter, A> {
        self.get_collection(&self.object.parameters)
    }
}
