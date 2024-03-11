use super::access::{Access, RoAccess};
use super::filebrowser::FileBrowser;
use crate::errors::{check, CubeError};
use crate::search::*;
use crate::types::{CubeUrl, FeedId, PipelineId, PluginId, PluginInstanceId};
use crate::{FeedResponse, LinkedModel, PipelineResponse, PluginInstanceResponse, PluginResponse};
use async_trait::async_trait;
use reqwest_middleware::ClientWithMiddleware;
use serde::de::DeserializeOwned;
use std::fmt::Display;

/// APIs you can interact with without having to log in.
#[async_trait]
pub trait BaseChrisClient<A: Access> {
    /// Get a filebrowser API client.
    fn filebrowser(&self) -> FileBrowser;

    /// Get the CUBE API URL.
    fn url(&self) -> &CubeUrl;

    /// Search for ChRIS plugins.
    fn plugin(&self) -> PluginSearchBuilder<A>;

    /// Search for pipeines.
    fn pipeline(&self) -> PipelineSearchBuilder<A>;

    /// Search for public feeds.
    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess>;

    // Note: get_feed and get_plugin_instance must be implemented manually,
    // whereas we can use a SearchBuilder for get_plugin and get_pipeline because
    // feeds and plugin instances are affected by the feature incompleteness of
    // public feeds.
    // See https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530

    /// Get a feed (directly).
    async fn get_feed(&self, id: FeedId) -> Result<LinkedModel<FeedResponse, A>, CubeError>;

    /// Get a plugin instance (directly).
    async fn get_plugin_instance(
        &self,
        id: PluginInstanceId,
    ) -> Result<LinkedModel<PluginInstanceResponse, A>, CubeError>;

    /// Get a plugin by ID
    async fn get_plugin(
        &self,
        id: PluginId,
    ) -> Result<LinkedModel<PluginResponse, A>, GetOnlyError> {
        self.plugin().id(id).search().page_limit(1).max_items(1).get_only().await
    }

    /// Get a pipeline by ID
    async fn get_pipeline(
        &self,
        id: PipelineId,
    ) -> Result<LinkedModel<PipelineResponse, A>, GetOnlyError> {
        self.pipeline().id(id).search().page_limit(1).max_items(1).get_only().await
    }
}

pub(crate) async fn fetch_id<A: Access, T: DeserializeOwned>(
    client: &ClientWithMiddleware,
    url: impl Display,
    id: u32,
) -> Result<LinkedModel<T, A>, CubeError> {
    let url = format!("{}{}/", url, id);
    let res = client.get(url).send().await?;
    let data = check(res).await?.json().await?;
    Ok(LinkedModel {
        client: client.clone(),
        object: data,
        phantom: Default::default(),
    })
}
