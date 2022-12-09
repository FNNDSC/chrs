use crate::common_types::CUBEApiUrl;
use crate::models::{AnyFilesUrl, PipelinesUrl, PluginParametersUrl, PluginsUrl};
use aliri_braid::braid;
use async_stream::stream;
use futures::Stream;
use reqwest::Url;
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
    url: Option<U>,
) -> impl Stream<Item = Result<R, reqwest::Error>> + 'a {
    // must have ownership of `url` because it might be a temporary value
    // inside a function that wants to return the stream created here
    stream! {
        let mut next_url = url.map(|i| i.clone());  // annoyingly necessary clone
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

impl PaginatedUrl for CUBEApiUrl {}
impl PaginatedUrl for AnyFilesUrl {}
impl PaginatedUrl for PluginsUrl {}
impl PaginatedUrl for PipelinesUrl {}
impl PaginatedUrl for SearchUrl {}
impl PaginatedUrl for PluginParametersUrl {}

/// Plugin meta URL.
#[braid(serde)]
pub struct SearchUrl;

impl From<Url> for SearchUrl {
    fn from(url: Url) -> Self {
        Self::from(url.as_ref())
    }
}

impl SearchUrl {
    pub(crate) fn of<T: Serialize + ?Sized>(
        u: &impl PaginatedUrl,
        query: &T,
    ) -> Result<Self, serde_urlencoded::ser::Error> {
        let mut url = Url::parse(u.as_ref()).unwrap().join("search/").unwrap();
        {
            let mut pairs = url.query_pairs_mut();
            let serializer = serde_urlencoded::Serializer::new(&mut pairs);
            query.serialize(serializer)?;
        }
        Ok(Self::from(url.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_url() {
        let example = "https://example.com/api/v1/plugins/";
        let expected = "https://example.com/api/v1/plugins/search/?name=dolphin";
        assert_eq!(
            SearchUrl::of(&PluginsUrl::from(example), &[("name", "dolphin")]),
            Ok(SearchUrl::from(expected))
        );
    }
}
