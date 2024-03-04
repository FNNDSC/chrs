use super::filebrowser::FileBrowser;
use super::searches::{FeedSearchBuilder, PluginSearchBuilder};
use super::variant::{Access, RoAccess};
use crate::types::CubeUrl;

/// APIs you can interact with without having to log in.
pub trait BaseChrisClient<V: Access> {
    /// Get a filebrowser API client.
    fn filebrowser(&self) -> FileBrowser;

    /// Get the CUBE API URL.
    fn url(&self) -> &CubeUrl;

    /// Search for ChRIS plugins.
    fn plugin(&self) -> PluginSearchBuilder<V>;

    /// Search for public feeds.
    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess>;
}
