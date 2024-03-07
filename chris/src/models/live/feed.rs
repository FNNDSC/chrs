use crate::errors::{check, CubeError};
use crate::models::data::FeedResponse;
use crate::search::SearchBuilder;
use crate::{Access, LinkedModel, NoteResponse, PluginInstanceResponse, RoAccess, RwAccess};

/// ChRIS feed note.
pub type Note<A> = LinkedModel<NoteResponse, A>;

/// ChRIS feed.
pub type Feed<A> = LinkedModel<FeedResponse, A>;

impl<A: Access> Feed<A> {
    /// Get the note of this feed.
    pub async fn get_note(&self) -> Result<Note<A>, CubeError> {
        let url = self.object.note.as_str();
        let res = self.client.get(url).send().await?;
        let object = check(res).await?.json().await?;
        Ok(LinkedModel {
            client: self.client.clone(),
            object,
            phantom: Default::default(),
        })
    }

    /// Get the plugin instances of this feed.
    pub fn get_plugin_instances(&self) -> FeedPluginInstances<A> {
        SearchBuilder::collection(&self.client, &self.object.plugin_instances)
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
        let res = self
            .client
            .put(self.object.url.as_str())
            .json(&[("name", &name)])
            .send()
            .await?;
        let data = check(res).await?.json().await?;
        Ok(Self {
            client: self.client.clone(),
            object: data,
            phantom: Default::default(),
        })
    }
}

type FeedPluginInstances<'a, A> = SearchBuilder<'a, PluginInstanceResponse, A>;
