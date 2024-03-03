use crate::types::CubeUrl;
use super::filebrowser::FileBrowser;

/// APIs you can interact with without having to log in.
pub trait PublicChrisClient {
    /// Get a filebrowser API client.
    fn filebrowser(&self) -> FileBrowser;

    fn url(&self) -> &CubeUrl;
}
