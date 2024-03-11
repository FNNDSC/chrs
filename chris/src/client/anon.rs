use async_trait::async_trait;
use reqwest::header::{HeaderMap, ACCEPT};
use serde::de::DeserializeOwned;

use crate::errors::{check, CubeError};
use crate::models::{BaseResponse, CubeLinks};
use crate::search::{
    FeedSearchBuilder, PipelineSearchBuilder, PluginSearchBuilder, QueryBuilder, LIMIT_ZERO,
};
use crate::types::*;
use crate::{FeedResponse, LinkedModel, PluginInstanceResponse};

use super::access::RoAccess;
use super::base::fetch_id;
use super::base::BaseChrisClient;
use super::filebrowser::FileBrowser;

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

impl AnonChrisClient {
    fn query<T: DeserializeOwned>(&self, url: &CollectionUrl) -> QueryBuilder<T, RoAccess> {
        QueryBuilder::query(self.client.clone(), url.clone())
    }
}

#[async_trait]
impl BaseChrisClient<RoAccess> for AnonChrisClient {
    fn filebrowser(&self) -> FileBrowser {
        FileBrowser::new(self.client.clone(), &self.links.filebrowser)
    }

    fn url(&self) -> &CubeUrl {
        &self.url
    }

    fn plugin(&self) -> PluginSearchBuilder<RoAccess> {
        self.query(&self.links.plugins)
    }

    fn pipeline(&self) -> PipelineSearchBuilder<RoAccess> {
        self.query(&self.links.pipelines)
    }

    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess> {
        self.query(&self.links.public_feeds)
    }

    async fn get_feed(&self, id: FeedId) -> Result<LinkedModel<FeedResponse, RoAccess>, CubeError> {
        fetch_id(&self.client, self.url(), id.0).await
    }

    async fn get_plugin_instance(
        &self,
        id: PluginInstanceId,
    ) -> Result<LinkedModel<PluginInstanceResponse, RoAccess>, CubeError> {
        fetch_id(&self.client, &self.links.plugin_instances, id.0).await
    }
}
