use super::access::{Access, RoAccess};
use super::filebrowser::FileBrowser;
use crate::errors::{check, CubeError};
use crate::search::{FeedSearchBuilder, PluginSearchBuilder};
use crate::types::{CubeUrl, FeedId, PluginInstanceId};
use crate::{FeedResponse, LinkedModel, PluginInstanceResponse};
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

    /// Search for public feeds.
    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess>;

    /// Get a feed (directly).
    async fn get_feed(&self, id: FeedId) -> Result<LinkedModel<FeedResponse, A>, CubeError>;

    /// Get a plugin instance (directly).
    async fn get_plugin_instance(
        &self,
        id: PluginInstanceId,
    ) -> Result<LinkedModel<PluginInstanceResponse, A>, CubeError>;
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
