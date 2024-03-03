use crate::client::search::LIMIT_ZERO;
use crate::client::searches::PluginSearchBuilder;
use crate::errors::{check, CubeError};
use crate::models::{AuthedPluginResponse, BaseResponse, CubeLinks};
use crate::types::*;
use crate::{BaseChrisClient, FileBrowser};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};

/// _ChRIS_ client with login.
#[derive(Debug)]
pub struct ChrisClient {
    client: reqwest::Client,
    url: CubeUrl,
    username: Username,
    links: CubeLinks,
}

impl ChrisClient {
    /// Connect to the ChRIS API using an authorization token.
    pub async fn connect(
        url: CubeUrl,
        username: Username,
        token: impl AsRef<str>,
    ) -> Result<Self, CubeError> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(token2header(token.as_ref()))
            .build()?;
        let res = client.get(url.as_str()).query(&LIMIT_ZERO).send().await?;
        let base_response: BaseResponse = check(res).await?.json().await?;
        Ok(ChrisClient {
            client,
            url,
            username,
            links: base_response.collection_links,
        })
    }
}

fn token2header(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let auth_data = format!("token {}", token);
    let mut value: HeaderValue = auth_data.parse().unwrap();
    value.set_sensitive(true);
    headers.insert(AUTHORIZATION, value);
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers
}

impl BaseChrisClient<AuthedPluginResponse> for ChrisClient {
    fn filebrowser(&self) -> FileBrowser {
        FileBrowser::new(self.client.clone(), &self.links.filebrowser)
    }

    fn url(&self) -> &CubeUrl {
        &self.url
    }

    fn plugin(&self) -> PluginSearchBuilder<AuthedPluginResponse> {
        PluginSearchBuilder::new(&self.client, &self.links.plugins)
    }
}
