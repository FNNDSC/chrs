use crate::errors::CubeError;
use crate::models::data::FeedResponse;
use crate::search::SearchBuilder;
use crate::{
    Access, LazyLinkedModel, LinkedModel, NoteResponse, PluginInstanceResponse, RoAccess, RwAccess,
};

/// ChRIS feed note.
pub type Note<A> = LinkedModel<NoteResponse, A>;
/// Similar to [Note] but without content.
pub type LazyNote<'a, A> = LazyLinkedModel<'a, NoteResponse, A>;

/// ChRIS feed.
pub type Feed<A> = LinkedModel<FeedResponse, A>;

impl<A: Access> Feed<A> {
    /// Get the note of this feed.
    pub fn note(&self) -> LazyLinkedModel<NoteResponse, A> {
        self.get_lazy(&self.object.note)
    }

    /// Get the plugin instances of this feed.
    pub fn get_plugin_instances(&self) -> FeedPluginInstances<A> {
        self.get_collection(&self.object.plugin_instances)
    }
}

impl<A: Access> Note<A> {
    /// Is the content of this note empty?
    pub fn is_empty(&self) -> bool {
        self.object.content.is_empty()
    }
}

/// A feed which you can edit.
pub type FeedRw = LinkedModel<FeedResponse, RwAccess>;

/// A feed which you can read but not edit.
pub type FeedRo = LinkedModel<FeedResponse, RoAccess>;

impl FeedRw {
    /// Set the name of a feed.
    pub async fn set_name(&self, name: String) -> Result<Self, CubeError> {
        self.put(&self.object.url, &[("name", &name)]).await
    }
}

type FeedPluginInstances<'a, A> = SearchBuilder<'a, PluginInstanceResponse, A>;
