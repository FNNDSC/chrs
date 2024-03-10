//! Helpers for pagination.

use crate::errors::{check, CubeError};
use crate::models::LinkedModel;
use crate::types::CollectionUrl;
use crate::Access;
use async_stream::{stream, try_stream};
use futures::Stream;
use reqwest_middleware::ClientWithMiddleware;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::marker::PhantomData;

/// An abstraction over collection APIs, i.e. paginated API endpoints which return a `results` list.
///
/// This is homologus to the Python implementation in aiochris:
///
/// <https://github.com/FNNDSC/aiochris/blob/adaff5bbc1d4d886ec2ca8155d82d266fa81d093/chris/util/search.py>
pub enum Search<R: DeserializeOwned, A: Access, Q: Serialize + Sized> {
    /// A search to CUBE, possibly containing [0, n) items.
    Search(ActualSearch<R, A, Q>),
    /// A search which cannot possibly contain items. It does not make requests to CUBE.
    Empty,
}

/// The "some" variant of [Search].
pub struct ActualSearch<R: DeserializeOwned, A: Access, Q: Serialize + Sized> {
    client: ClientWithMiddleware,
    base_url: String,
    query: Q,
    phantom: PhantomData<(R, A)>,

    /// Maximum number of items to produce
    max_items: Option<usize>,

    /// Whether to append "search/" to the URL.
    is_search: bool,
}

impl<R: DeserializeOwned, A: Access, Q: Serialize + Sized> ActualSearch<R, A, Q> {
    /// Create a HTTP GET request for this search.
    fn get_search(&self) -> reqwest_middleware::RequestBuilder {
        if self.is_search {
            let url = format!("{}search/", &self.base_url);
            self.client.get(url).query(&self.query)
        } else {
            let url = self.base_url.as_str();
            self.client.get(url)
        }
    }

    /// See [Search::get_count]
    async fn get_count(&self) -> Result<u32, CubeError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let data: HasCount = check(res).await?.json().await?;
        Ok(data.count)
    }

    /// See [Search::get_first]
    async fn get_first(&self) -> Result<Option<LinkedModel<R, A>>, CubeError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let page: Paginated<R> = check(res).await?.json().await?;
        let first = page.results.into_iter().next();
        let ret = first.map(|data| LinkedModel {
            client: self.client.clone(),
            object: data,
            phantom: Default::default(),
        });
        Ok(ret)
    }

    /// See [Search::get_only]
    async fn get_only(&self) -> Result<LinkedModel<R, A>, GetOnlyError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let page: Paginated<R> = check(res).await?.json().await?;

        if page.count > 1 {
            return Err(GetOnlyError::MoreThanOne);
        }

        if let Some(data) = page.results.into_iter().next() {
            Ok(LinkedModel {
                client: self.client.clone(),
                object: data,
                phantom: Default::default(),
            })
        } else {
            Err(GetOnlyError::None)
        }
    }

    /// See [Search::stream]
    fn stream(&self) -> impl Stream<Item = Result<R, CubeError>> + '_ {
        let mut count = 0;
        try_stream! {
            // retrieval of the first page works a little differently, since we
            // don't know what `next_url` is, we call client.get(...).query(...)
            // instead of client.get(next_url)
            let res = self.get_search().send().await?;
            let page: Paginated<R> = check(res).await?.json().await?;
            for item in page.results {
                if count >= self.max_items.unwrap_or(usize::MAX) {
                    return;
                }
                yield item;
                count += 1;
            }

            let mut next_url = page.next;
            // subsequent pages after the first are retrieved using a loop.
            while let Some(u) = next_url {
                let res = self.client.get(&u).send().await?;
                let page: Paginated<R> = check(res).await?.json().await?;

                for item in page.results {
                if count >= self.max_items.unwrap_or(usize::MAX) {
                        return;
                    }
                    yield item;
                    count += 1;
                }
                next_url = page.next;
            }
        }
    }
}

impl<R: DeserializeOwned, A: Access, Q: Serialize + Sized> Search<R, A, Q> {
    fn new(
        client: ClientWithMiddleware,
        base_url: CollectionUrl,
        query: Q,
        max_items: Option<usize>,
        is_search: bool,
    ) -> Self {
        let s = ActualSearch {
            client,
            base_url: base_url.take(),
            query,
            is_search,
            max_items,
            phantom: Default::default(),
        };
        Self::Search(s)
    }

    /// Create a search query.
    #[allow(clippy::self_named_constructors)]
    pub(crate) fn search(
        client: ClientWithMiddleware,
        base_url: CollectionUrl,
        query: Q,
        max_items: Option<usize>,
    ) -> Self {
        Self::new(client, base_url, query, max_items, true)
    }

    /// Constructor for retrieving items from the given `base_url` itself
    /// (instead of `{base_url}search/`), without any query parameters.
    pub(crate) fn collection(
        client: ClientWithMiddleware,
        base_url: CollectionUrl,
        query: Q,
        max_items: Option<usize>,
    ) -> Self {
        Self::new(client, base_url, query, max_items, false)
    }

    /// Get the count of items in this collection.
    pub async fn get_count(&self) -> Result<u32, CubeError> {
        match self {
            Self::Search(s) => s.get_count().await,
            Self::Empty => Ok(0),
        }
    }
}

impl<R: DeserializeOwned, A: Access, Q: Serialize + Sized> Search<R, A, Q> {
    /// Get the first item from this collection.
    ///
    /// See also: [Search::get_only]
    pub async fn get_first(&self) -> Result<Option<LinkedModel<R, A>>, CubeError> {
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
    pub async fn get_only(&self) -> Result<LinkedModel<R, A>, GetOnlyError> {
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
    pub fn stream_connected(
        &self,
    ) -> impl Stream<Item = Result<LinkedModel<R, A>, CubeError>> + '_ {
        try_stream! {
            match self {
                Search::Search(s) => {
                    for await item in s.stream() {
                        yield LinkedModel { client: s.client.clone(), object: item?, phantom: Default::default() }
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
    #[allow(unused)]
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

impl From<reqwest_middleware::Error> for GetOnlyError {
    fn from(error: reqwest_middleware::Error) -> Self {
        CubeError::from(error).into()
    }
}

impl From<reqwest::Error> for GetOnlyError {
    fn from(error: reqwest::Error) -> Self {
        CubeError::from(error).into()
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
