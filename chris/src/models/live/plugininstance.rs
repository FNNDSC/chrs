use crate::{
    Access, FeedResponse, LazyLinkedModel, LinkedModel, PluginInstanceResponse, RoAccess, RwAccess,
};

pub type PluginInstanceRw = LinkedModel<PluginInstanceResponse, RwAccess>;
pub type PluginInstanceRo = LinkedModel<PluginInstanceResponse, RoAccess>;

impl<A: Access> LinkedModel<PluginInstanceResponse, A> {
    /// Feed of this plugin instance.
    pub fn feed(&self) -> LazyLinkedModel<FeedResponse, A> {
        LazyLinkedModel {
            url: &self.object.feed,
            client: self.client.clone(),
            phantom: Default::default(),
        }
    }
}
