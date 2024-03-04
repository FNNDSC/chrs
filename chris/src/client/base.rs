use super::filebrowser::FileBrowser;
use super::searches::{FeedSearchBuilder, PluginSearchBuilder};
use super::variant::{Access, RoAccess};
use crate::errors::{check, CubeError};
use crate::types::{CubeUrl, FeedId, PluginInstanceId};
use crate::{FeedResponse, LinkedModel, PluginInstanceResponse};
use reqwest_middleware::ClientWithMiddleware;
use serde::de::DeserializeOwned;
use std::fmt::Display;
use std::future::Future;

/// APIs you can interact with without having to log in.
pub trait BaseChrisClient<A: Access + Sync> {
    /// Get a filebrowser API client.
    fn filebrowser(&self) -> FileBrowser;

    /// Get the CUBE API URL.
    fn url(&self) -> &CubeUrl;

    /// Search for ChRIS plugins.
    fn plugin(&self) -> PluginSearchBuilder<A>;

    /// Search for public feeds.
    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess>;

    /// Get a feed (directly).
    fn get_feed(
        &self,
        id: FeedId,
    ) -> impl Future<Output = Result<LinkedModel<FeedResponse, A>, CubeError>> + Send;

    /// Get a plugin instance (directly).
    fn get_plugin_instance(
        &self,
        id: PluginInstanceId,
    ) -> impl Future<Output = Result<LinkedModel<PluginInstanceResponse, A>, CubeError>> + Send;
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
