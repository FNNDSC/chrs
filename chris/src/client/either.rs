use crate::errors::CubeError;
use crate::search::{FeedSearchBuilder, PipelineSearchBuilder, PluginSearchBuilder};
use crate::types::{CubeUrl, FeedId, PluginInstanceId, Username};
use crate::{
    AnonChrisClient, BaseChrisClient, ChrisClient, FeedResponse, FileBrowser, LinkedModel,
    PluginInstanceResponse, RoAccess,
};
use async_trait::async_trait;

/// Either an anonymous client or a logged in user. A shoddy workaround for combining how enums
/// work and how [AccessRw] and [AccessRo] could be represented using an enum.
pub enum EitherClient {
    Anon(AnonChrisClient),
    LoggedIn(ChrisClient),
}

/// A client which accesses read-only APIs only.
/// It may use authorization, in which case it is able to read private collections.
pub type RoClient = Box<dyn BaseChrisClient<RoAccess>>;

impl EitherClient {
    /// Use this client for public read-only access only.
    pub fn into_ro(self) -> RoClient {
        match self {
            Self::Anon(c) => Box::new(c),
            Self::LoggedIn(c) => Box::new(c.into_ro()),
        }
    }

    /// Get the username of this client.
    pub fn username(&self) -> Option<&Username> {
        match self {
            Self::Anon(_) => None,
            Self::LoggedIn(c) => Some(c.username()),
        }
    }

    /// Get the client if this is logged in.
    pub fn logged_in(self) -> Option<ChrisClient> {
        match self {
            Self::LoggedIn(c) => Some(c),
            _ => None,
        }
    }

    /// Get the client if this is logged in.
    pub fn logged_in_ref(&self) -> Option<&ChrisClient> {
        match self {
            Self::LoggedIn(c) => Some(c),
            _ => None,
        }
    }
}

#[async_trait]
impl BaseChrisClient<RoAccess> for EitherClient {
    fn filebrowser(&self) -> FileBrowser {
        match self {
            Self::Anon(c) => c.filebrowser(),
            Self::LoggedIn(c) => c.filebrowser(),
        }
    }

    fn url(&self) -> &CubeUrl {
        match self {
            Self::Anon(c) => c.url(),
            Self::LoggedIn(c) => c.url(),
        }
    }

    fn plugin(&self) -> PluginSearchBuilder<RoAccess> {
        match self {
            Self::Anon(c) => c.plugin(),
            Self::LoggedIn(c) => c.plugin().into_ro(),
        }
    }

    fn pipeline(&self) -> PipelineSearchBuilder<RoAccess> {
        match self {
            Self::Anon(c) => c.pipeline(),
            Self::LoggedIn(c) => c.pipeline().into_ro(),
        }
    }

    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess> {
        match self {
            Self::Anon(c) => c.public_feeds(),
            Self::LoggedIn(c) => c.public_feeds(),
        }
    }

    async fn get_feed<'a>(
        &'a self,
        id: FeedId,
    ) -> Result<LinkedModel<FeedResponse, RoAccess>, CubeError> {
        match self {
            Self::Anon(c) => c.get_feed(id).await,
            Self::LoggedIn(c) => c.get_feed(id).await.map(|f| f.into()),
        }
    }

    async fn get_plugin_instance(
        &self,
        id: PluginInstanceId,
    ) -> Result<LinkedModel<PluginInstanceResponse, RoAccess>, CubeError> {
        match self {
            Self::Anon(c) => c.get_plugin_instance(id).await,
            Self::LoggedIn(c) => c.get_plugin_instance(id).await.map(|p| p.into()),
        }
    }
}
