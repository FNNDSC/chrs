use crate::errors::{check, CubeError};
use crate::models::data::FeedResponse;
use crate::{LinkedModel, RoAccess, RwAccess};
//
// pub struct ShallowFeed {
//     client: Client,
//     pub url: FeedUrl,
// }
//
// impl ShallowFeed {
//     pub(crate) fn new(client: Client, url: FeedUrl) -> Self {
//         Self { client, url }
//     }
//
//     pub async fn set_name(&self, name: &str) -> Result<FeedResponse, CUBEError> {
//         let res = self
//             .client
//             .put(self.url.as_str())
//             .json(&SetFeedNameBody { name })
//             .send()
//             .await?;
//         Ok(check(res).await?.json().await?)
//     }
// }

/// A feed which you can edit.
pub type FeedRw = LinkedModel<FeedResponse, RwAccess>;

/// A feed which you can read but not edit.
pub type FeedRo = LinkedModel<FeedResponse, RoAccess>;

impl FeedRw {
    /// Set the name of a feed.
    pub async fn set_name(&self, name: String) -> Result<Self, CubeError> {
        let res = self
            .client
            .put(self.object.url.as_str())
            .json(&[("name", &name)])
            .send()
            .await?;
        let data = check(res).await?.json().await?;
        Ok(Self {
            client: self.client.clone(),
            object: data,
            phantom: Default::default(),
        })
    }
}
