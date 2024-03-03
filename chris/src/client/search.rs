//! Helpers for pagination.

use crate::errors::{check, CubeError};
use crate::models::LinkedModel;
use async_stream::{stream, try_stream};
use futures::Stream;
use reqwest::RequestBuilder;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;

/// An abstraction over collection APIs, i.e. paginated API endpoints which return a `results` list.
///
/// This is homologus to the Python implementation in aiochris:
///
/// <https://github.com/FNNDSC/aiochris/blob/adaff5bbc1d4d886ec2ca8155d82d266fa81d093/chris/util/search.py>
pub enum Search<R: DeserializeOwned, Q: Serialize + Sized> {
    /// A search to CUBE, possibly containing [0, n) items.
    Search(ActualSearch<R, Q>),
    /// A search which cannot possibly contain items. It does not make requests to CUBE.
    Empty,
}

/// The "some" variant of [Search].
pub struct ActualSearch<R: DeserializeOwned, Q: Serialize + Sized> {
    client: reqwest::Client,
    base_url: String,
    query: Q,
    phantom: PhantomData<R>,

    // The perfectionist approach would be to define another enum variant,
    // the least-code approach would be to use `dyn`
    /// Bad-ish boolean.
    basic: bool,
}

impl<R: DeserializeOwned, Q: Serialize + Sized> ActualSearch<R, Q> {
    /// See [Search::get_count]
    async fn get_count(&self) -> Result<u32, CubeError> {
        let res = self
            .client
            .get(&self.base_url)
            .query(&self.query)
            .query(&LIMIT_ZERO)
            .send()
            .await?;
        let data: HasCount = check(res).await?.json().await?;
        Ok(data.count)
    }
}

impl<R: DeserializeOwned, Q: Serialize + Sized> ActualSearch<R, Q> {
    /// Create a HTTP GET request for this search.
    fn get_search(&self) -> RequestBuilder {
        if self.basic {
            let url = self.base_url.as_str();
            self.client.get(url)
        } else {
            let url = format!("{}search/", &self.base_url);
            self.client.get(url).query(&self.query)
        }
    }

    /// See [Search::get_first]
    async fn get_first(&self) -> Result<Option<LinkedModel<R>>, CubeError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let page: Paginated<R> = check(res).await?.json().await?;
        let first = page.results.into_iter().next();
        let ret = first.map(|data| LinkedModel {
            client: self.client.clone(),
            object: data,
        });
        Ok(ret)
    }

    /// See [Search::get_only]
    async fn get_only(&self) -> Result<LinkedModel<R>, GetOnlyError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let page: Paginated<R> = check(res).await?.json().await?;

        if page.count > 1 {
            return Err(GetOnlyError::MoreThanOne);
        }

        if let Some(data) = page.results.into_iter().next() {
            Ok(LinkedModel {
                client: self.client.clone(),
                object: data,
            })
        } else {
            Err(GetOnlyError::None)
        }
    }

    /// See [Search::stream]
    fn stream(&self) -> impl Stream<Item = Result<R, CubeError>> + '_ {
        try_stream! {
            // retrieval of the first page works a little differently, since we
            // don't know what `next_url` is, we call client.get(...).query(...)
            // instead of client.get(next_url)
            let res = self.get_search().send().await?;
            let page: Paginated<R> = check(res).await?.json().await?;
            for item in page.results {
                yield item
            }

            let mut next_url = page.next;
            // subsequent pages after the first are retrieved using a loop.
            while let Some(u) = next_url {
                let res = self.client.get(&u).send().await?;
                let page: Paginated<R> = check(res).await?.json().await?;

                for item in page.results {
                    yield item
                }
                next_url = page.next;
            }
        }
    }
}

impl<R: DeserializeOwned> Search<R, ()> {
    /// Constructor for retrieving items from the given `base_url` itself
    /// (instead of `{base_url}search/`), without any query parameters.
    pub(crate) fn basic(client: &reqwest::Client, base_url: impl ToString) -> Self {
        let s = ActualSearch {
            client: client.clone(),
            base_url: base_url.to_string(),
            query: (),
            phantom: Default::default(),
            basic: true,
        };
        Self::Search(s)
    }
}

impl<R: DeserializeOwned, Q: Serialize + Sized> Search<R, Q> {
    /// Create a search query.
    pub(crate) fn new(client: &reqwest::Client, base_url: impl ToString, query: Q) -> Self {
        let s = ActualSearch {
            client: client.clone(),
            base_url: base_url.to_string(),
            query,
            phantom: Default::default(),
            basic: false,
        };
        Self::Search(s)
    }

    /// Create a dummy empty search object which returns no results.
    pub(crate) fn empty() -> Self {
        Self::Empty
    }

    /// Get the count of items in this collection.
    pub async fn get_count(&self) -> Result<u32, CubeError> {
        match self {
            Self::Search(s) => s.get_count().await,
            Self::Empty => Ok(0),
        }
    }
}

impl<R: DeserializeOwned, Q: Serialize + Sized> Search<R, Q> {
    /// Get the first item from this collection.
    ///
    /// See also: [Search::get_only]
    pub async fn get_first(&self) -> Result<Option<LinkedModel<R>>, CubeError> {
        match self {
            Search::Search(s) => s.get_first().await,
            Search::Empty => Ok(None),
        }
    }

    /// Get the _only_ item from this collection.
    ///
    /// This function _should_ only be called when some invariant holds that
    /// the collection has only one item, e.g. searching for plugins giving
    /// both `name` and `version`, or searching for anything giving `id`.
    pub async fn get_only(&self) -> Result<LinkedModel<R>, GetOnlyError> {
        match self {
            Search::Search(s) => s.get_only().await,
            Search::Empty => Err(GetOnlyError::None),
        }
    }

    /// Produce items from this collection. Pagination is handled transparently,
    /// i.e. HTTP GET requests are sent as-needed.
    pub fn stream(&self) -> impl Stream<Item = Result<R, CubeError>> + '_ {
        stream! {
            match self {
                Search::Search(s) => {
                    for await item in s.stream() {
                        yield item
                    }
                }
                Search::Empty => {}
            }
        }
    }

    /// Like [Self::stream], but clones the client for each item, so that methods can be called
    /// on the returned items.
    pub fn stream_connected(&self) -> impl Stream<Item = Result<LinkedModel<R>, CubeError>> + '_ {
        try_stream! {
            match self {
                Search::Search(s) => {
                    for await item in s.stream() {
                        yield LinkedModel { client: s.client.clone(), object: item? }
                    }
                }
                Search::Empty => {}
            }
        }
    }
}

/// Generic response from paginated endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct Paginated<R> {
    pub count: u32,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<R>,
}

/// Errors for [Search::get_only].
#[derive(thiserror::Error, Debug)]
pub enum GetOnlyError {
    #[error("Empty collection")]
    None,
    #[error("More than one result in collection")]
    MoreThanOne,
    #[error(transparent)]
    Error(#[from] CubeError),
}

impl From<reqwest::Error> for GetOnlyError {
    fn from(value: reqwest::Error) -> Self {
        CubeError::Raw(value).into()
    }
}

/// Query string parameters for paginated GET endpoints.
#[derive(Serialize)]
pub(crate) struct PaginationQuery {
    pub limit: u8,
    pub offset: u32,
}

pub(crate) const LIMIT_ZERO: PaginationQuery = PaginationQuery {
    limit: 0,
    offset: 0,
};

pub(crate) const LIMIT_ONE: PaginationQuery = PaginationQuery {
    limit: 1,
    offset: 0,
};

/// A HTTP JSON response which has a count field.
#[derive(Deserialize)]
struct HasCount {
    count: u32,
}
