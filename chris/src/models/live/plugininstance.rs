use crate::search::SearchBuilder;
use crate::{
    Access, FeedResponse, LazyLinkedModel, LinkedModel, PluginInstanceParameterResponse,
    PluginInstanceResponse, PluginParameter, RoAccess, RwAccess,
};

pub type PluginInstanceRw = LinkedModel<PluginInstanceResponse, RwAccess>;
pub type PluginInstanceRo = LinkedModel<PluginInstanceResponse, RoAccess>;

impl<A: Access> LinkedModel<PluginInstanceResponse, A> {
    /// Feed of this plugin instance.
    pub fn feed(&self) -> LazyLinkedModel<FeedResponse, A> {
        self.get_lazy(&self.object.feed)
    }

    pub fn parameters(&self) -> SearchBuilder<PluginInstanceParameterResponse, A> {
        self.get_collection(&self.object.parameters)
    }
}

pub type PluginInstanceParameter<A> = LinkedModel<PluginInstanceParameterResponse, A>;

impl<A: Access> PluginInstanceParameter<A> {
    pub fn plugin_parameter(&self) -> LazyLinkedModel<PluginParameter, A> {
        self.get_lazy(&self.object.plugin_param)
    }
}
