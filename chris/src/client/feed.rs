use crate::errors::{check, CUBEError};
use crate::models::{FeedResponse, FeedUrl};
use reqwest::Client;
use serde::Serialize;

pub struct ShallowFeed {
    client: Client,
    pub url: FeedUrl,
}

impl ShallowFeed {
    pub(crate) fn new(client: Client, url: FeedUrl) -> Self {
        Self { client, url }
    }

    pub async fn set_name(&self, name: &str) -> Result<FeedResponse, CUBEError> {
        let res = self
            .client
            .put(self.url.as_str())
            .json(&SetFeedNameBody { name })
            .send()
            .await?;
        Ok(check(res).await?.json().await?)
    }
}

#[derive(Serialize)]
struct SetFeedNameBody<'a> {
    name: &'a str,
}
