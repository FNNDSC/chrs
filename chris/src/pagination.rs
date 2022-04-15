use crate::api::AnyFilesUrl;
use async_stream::stream;
use futures::Stream;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Query string parameters for paginated GET endpoints.
#[derive(Serialize)]
pub(crate) struct PaginationQuery {
    pub limit: u8,
    pub offset: u32,
}

/// An API URL which returns a paginated resource.
pub(crate) trait PaginatedUrl: AsRef<str> + Clone + DeserializeOwned {}

/// Create a [futures::Stream] that yields items from a paginated URL.
///
/// Limitation: Cannot produce [crate::client::CUBEError]
pub(crate) fn paginate<'a, U: 'a + PaginatedUrl, R: 'a + DeserializeOwned>(
    client: &'a reqwest::Client,
    url: Option<&'a U>,
) -> impl Stream<Item = Result<R, reqwest::Error>> + 'a {
    stream! {
        let mut next_url = url.map(|i| i.clone());
        while let Some(u) = next_url {
            let res = client.get(u.as_ref()).send().await?;
            let page: Paginated<U, R> = res.json().await?;

            for item in page.results {
                yield Ok(item);
            }
            next_url = page.next;
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct Paginated<U, T> {
    pub count: u32,
    pub next: Option<U>,
    pub previous: Option<U>,
    pub results: Vec<T>,
}

impl PaginatedUrl for AnyFilesUrl {}
