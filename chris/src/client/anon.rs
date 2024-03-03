use super::filebrowser::FileBrowser;
use crate::client::base::BaseChrisClient;
use crate::client::search::LIMIT_ZERO;
use crate::client::searches::PluginSearchBuilder;
use crate::errors::{check, CubeError};
use crate::models::{AnonPluginResponse, BaseResponse, CubeLinks};
use crate::types::*;
use reqwest::header::{HeaderMap, ACCEPT};

/// Anonymous ChRIS client.
pub struct AnonChrisClient {
    client: reqwest::Client,
    url: CubeUrl,
    links: CubeLinks,
}

impl AnonChrisClient {
    /// Connect to the ChRIS API.
    pub async fn connect(url: CubeUrl) -> Result<Self, CubeError> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(accept_json())
            .build()?;
        let res = client.get(url.as_str()).query(&LIMIT_ZERO).send().await?;
        let base_response: BaseResponse = check(res).await?.json().await?;
        Ok(Self {
            client,
            url,
            links: base_response.collection_links,
        })
    }
}

fn accept_json() -> HeaderMap {
    HeaderMap::from_iter([(ACCEPT, "application/json".parse().unwrap())].into_iter())
}

impl BaseChrisClient<AnonPluginResponse> for AnonChrisClient {
    fn filebrowser(&self) -> FileBrowser {
        FileBrowser::new(self.client.clone(), &self.links.filebrowser)
    }

    fn url(&self) -> &CubeUrl {
        &self.url
    }

    fn plugin(&self) -> PluginSearchBuilder<AnonPluginResponse> {
        PluginSearchBuilder::new(&self.client, &self.links.plugins)
    }
}
