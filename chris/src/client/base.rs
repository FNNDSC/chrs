use super::filebrowser::FileBrowser;
use crate::client::searches::PluginSearchBuilder;
use crate::types::CubeUrl;
use serde::de::DeserializeOwned;

/// APIs you can interact with without having to log in.
pub trait BaseChrisClient<P: DeserializeOwned> {
    /// Get a filebrowser API client.
    fn filebrowser(&self) -> FileBrowser;

    /// Get the CUBE API URL.
    fn url(&self) -> &CubeUrl;

    /// Search for ChRIS plugins.
    fn plugin(&self) -> PluginSearchBuilder<P>;
}
