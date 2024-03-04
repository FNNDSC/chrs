use super::filebrowser::FileBrowser;
use super::searches::{FeedSearchBuilder, PluginSearchBuilder};
use super::variant::{Access, RoAccess};
use crate::errors::{check, CubeError};
use crate::types::{CubeUrl, FeedId};
use crate::{FeedResponse, LinkedModel};
use reqwest_middleware::ClientWithMiddleware;

/// APIs you can interact with without having to log in.
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
}

pub(crate) async fn get_feed<A: Access>(
    client: &ClientWithMiddleware,
    url: &CubeUrl,
    id: FeedId,
) -> Result<LinkedModel<FeedResponse, A>, CubeError> {
    let url = format!("{}{}/", url, id.0);
    let res = client.get(url).send().await?;
    let data = check(res).await?.json().await?;
    Ok(LinkedModel {
        client: client.clone(),
        object: data,
        phantom: Default::default(),
    })
}
