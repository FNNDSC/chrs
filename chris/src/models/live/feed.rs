use serde_with::serde_derive::Serialize;

use crate::errors::CubeError;
use crate::models::data::FeedResponse;
use crate::search::Search;
use crate::{
    Access, BasicFileResponse, LazyLinkedModel, LinkedModel, NoteResponse, PluginInstanceResponse,
    RoAccess, RwAccess,
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
    pub fn get_plugin_instances(&self) -> Search<PluginInstanceResponse, A> {
        self.get_collection(&self.object.plugin_instances)
    }

    /// Get files of this feed.
    pub fn files(&self) -> Search<BasicFileResponse, A> {
        self.get_collection(&self.object.files)
    }
}

impl<A: Access> Note<A> {
    /// Is the content of this note empty?
    pub fn is_empty(&self) -> bool {
        self.object.content.is_empty()
    }
}

impl Note<RwAccess> {
    /// Set the note
    pub async fn set(
        self,
        title: impl AsRef<str>,
        content: impl AsRef<str>,
    ) -> Result<Note<RwAccess>, CubeError> {
        let body = NoteRequest {
            title: title.as_ref(),
            content: content.as_ref(),
        };
        self.put(&self.object.url, &body).await
    }
}

impl LazyNote<'_, RwAccess> {
    /// Set the note
    pub async fn set(
        self,
        title: impl AsRef<str>,
        content: impl AsRef<str>,
    ) -> Result<Note<RwAccess>, CubeError> {
        let body = NoteRequest {
            title: title.as_ref(),
            content: content.as_ref(),
        };
        self.put(&body).await
    }
}

/// A feed which you can edit.
pub type FeedRw = LinkedModel<FeedResponse, RwAccess>;

/// A feed which you can read but not edit.
pub type FeedRo = LinkedModel<FeedResponse, RoAccess>;

/// A lazy feed object.
pub type LazyFeed<'a, A> = LazyLinkedModel<'a, FeedResponse, A>;

/// A lazy feed object you can edit.
pub type LazyFeedRw<'a> = LazyFeed<'a, RwAccess>;

impl FeedRw {
    /// Set the name of a feed.
    pub async fn set_name(&self, name: &str) -> Result<Self, CubeError> {
        self.put(&self.object.url, &Name { name }).await
    }
}

impl<'a> LazyFeedRw<'a> {
    /// Set the name of a feed.
    pub async fn set_name(&self, name: &str) -> Result<FeedRw, CubeError> {
        self.put(&Name { name }).await
    }
}

#[derive(Serialize)]
struct Name<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct NoteRequest<'a> {
    title: &'a str,
    content: &'a str,
}
