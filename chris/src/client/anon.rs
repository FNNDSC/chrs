use super::filebrowser::FileBrowser;
use crate::client::base::BaseChrisClient;
use crate::client::search::LIMIT_ZERO;
use crate::client::searches::{FeedSearchBuilder, PluginSearchBuilder};
use crate::client::variant::RoAccess;
use crate::errors::{check, CubeError};
use crate::models::{BaseResponse, CubeLinks, PluginResponse};
use crate::types::*;
use reqwest::header::{HeaderMap, ACCEPT};

/// Anonymous ChRIS client.
pub struct AnonChrisClient {
    client: reqwest_middleware::ClientWithMiddleware,
    url: CubeUrl,
    links: CubeLinks,
}

pub struct AnonChrisClientBuilder {
    url: CubeUrl,
    builder: reqwest_middleware::ClientBuilder,
}

impl AnonChrisClientBuilder {
    pub(crate) fn new(url: CubeUrl) -> Result<Self, reqwest::Error> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(accept_json())
            .build()?;
        let builder = reqwest_middleware::ClientBuilder::new(client);
        Ok(Self { url, builder })
    }

    /// Add middleware to the HTTP client.
    pub fn with<M: reqwest_middleware::Middleware>(self, middleware: M) -> Self {
        Self {
            url: self.url,
            builder: self.builder.with(middleware),
        }
    }

    /// Connect to the ChRIS API.
    pub async fn connect(self) -> Result<AnonChrisClient, CubeError> {
        let client = self.builder.build();
        let res = client
            .get(self.url.as_str())
            .query(&LIMIT_ZERO)
            .send()
            .await?;
        let base_response: BaseResponse = check(res).await?.json().await?;
        Ok(AnonChrisClient {
            client,
            url: self.url,
            links: base_response.collection_links,
        })
    }
}

impl AnonChrisClient {
    /// Create a client builder.
    pub fn build(url: CubeUrl) -> Result<AnonChrisClientBuilder, reqwest::Error> {
        AnonChrisClientBuilder::new(url)
    }
}

fn accept_json() -> HeaderMap {
    HeaderMap::from_iter([(ACCEPT, "application/json".parse().unwrap())])
}

impl BaseChrisClient<RoAccess> for AnonChrisClient {
    fn filebrowser(&self) -> FileBrowser {
        FileBrowser::new(self.client.clone(), &self.links.filebrowser)
    }

    fn url(&self) -> &CubeUrl {
        &self.url
    }

    fn plugin(&self) -> PluginSearchBuilder<RoAccess> {
        PluginSearchBuilder::new(&self.client, &self.links.plugins)
    }

    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess> {
        FeedSearchBuilder::new(&self.client, &self.links.public_feeds)
    }
}
