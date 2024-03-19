//! Helpers for pagination.

use crate::errors::{check, CubeError};
use crate::models::LinkedModel;
use crate::types::CollectionUrl;
use crate::{Access, RoAccess, RwAccess};
use async_stream::{stream, try_stream};
use futures::Stream;
use reqwest_middleware::ClientWithMiddleware;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

/// An abstraction over collection APIs, i.e. paginated API endpoints which return a `results` list.
///
/// This is homologus to the Python implementation in aiochris:
///
/// <https://github.com/FNNDSC/aiochris/blob/adaff5bbc1d4d886ec2ca8155d82d266fa81d093/chris/util/search.py>
pub struct Search<R: DeserializeOwned, A: Access> {
    actual: Option<ActualSearch<R, A>>,
    max_items: Option<usize>,
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum QueryValue {
    U32(u32),
    String(String),
}

/// Implementation of [Search]
struct ActualSearch<R: DeserializeOwned, A: Access> {
    client: ClientWithMiddleware,
    base_url: CollectionUrl,
    query: HashMap<&'static str, QueryValue>,
    phantom: PhantomData<(R, A)>,

    /// Whether to append "search/" to the URL.
    is_search: bool,
}

impl<R: DeserializeOwned, A: Access> ActualSearch<R, A> {
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

    fn page_limit(mut self, limit: u32) -> Self {
        self.query.insert("limit", QueryValue::U32(limit));
        self
    }

    /// See [Search::downgrade]
    fn downgrade<T: DeserializeOwned>(self) -> ActualSearch<T, A> {
        ActualSearch {
            client: self.client,
            base_url: self.base_url,
            query: self.query,
            phantom: Default::default(),
            is_search: self.is_search,
        }
    }

    /// See [Search::get_count]
    async fn get_count(&self) -> Result<usize, CubeError> {
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
        try_stream! {
            // retrieval of the first page works a little differently, since we
            // don't know what `next_url` is, we call client.get(...).query(...)
            // instead of client.get(next_url)
            let res = self.get_search().send().await?;
            let page: Paginated<R> = check(res).await?.json().await?;
            for item in page.results {
                yield item;
            }

            let mut next_url = page.next;
            // subsequent pages after the first are retrieved using a loop.
            while let Some(u) = next_url {
                let res = self.client.get(&u).send().await?;
                let page: Paginated<R> = check(res).await?.json().await?;

                for item in page.results {
                    yield item;
                }
                next_url = page.next;
            }
        }
    }
}

impl<R: DeserializeOwned, A: Access> Search<R, A> {
    fn new(
        client: ClientWithMiddleware,
        base_url: CollectionUrl,
        query: HashMap<&'static str, QueryValue>,
        is_search: bool,
    ) -> Self {
        let actual = ActualSearch {
            client,
            base_url,
            query,
            is_search,
            phantom: Default::default(),
        };
        Self {
            actual: Some(actual),
            max_items: None,
        }
    }

    /// Create a search for a search api, e.g. `{base_url}search/`.
    pub(crate) fn with_query(
        client: ClientWithMiddleware,
        base_url: CollectionUrl,
        query: HashMap<&'static str, QueryValue>,
    ) -> Self {
        Self::new(client, base_url, query, true)
    }

    /// Constructor for retrieving items from the given `base_url` itself
    /// (instead of `{base_url}search/`), without any query parameters.
    pub(crate) fn collection(client: ClientWithMiddleware, base_url: CollectionUrl) -> Self {
        Self::new(client, base_url, HashMap::with_capacity(0), false)
    }

    /// Create an empty search
    pub fn empty() -> Self {
        Self {
            actual: None,
            max_items: None,
        }
    }

    /// Convert the yield type.
    ///
    /// `T` _must_ be a subset of `R`.
    pub(crate) fn downgrade<T: DeserializeOwned>(self) -> Search<T, A> {
        Search {
            actual: self.actual.map(|a| a.downgrade()),
            max_items: self.max_items,
        }
    }

    /// Set the maximum number of items to retrieve per request.
    /// (This value is for performance tuning.)
    ///
    /// See also: [Self::max_items]
    pub fn page_limit(self, limit: u32) -> Self {
        Self {
            actual: self.actual.map(|a| a.page_limit(limit)),
            ..self
        }
    }

    /// Set the maximum number of items to yield.
    ///
    /// See also: [Self::page_limit]
    pub fn max_items(self, max: usize) -> Self {
        Self {
            max_items: Some(max),
            ..self
        }
    }

    /// Get the count of items in this collection.
    pub async fn get_count(&self) -> Result<usize, CubeError> {
        if let Some(search) = &self.actual {
            search.get_count().await
        } else {
            Ok(0)
        }
    }

    /// Get the first item from this collection.
    ///
    /// See also: [Search::get_only]
    pub async fn get_first(&self) -> Result<Option<LinkedModel<R, A>>, CubeError> {
        if let Some(search) = &self.actual {
            search.get_first().await
        } else {
            Ok(None)
        }
    }

    /// Get the _only_ item from this collection.
    ///
    /// This function _should_ only be called when some invariant holds that
    /// the collection has only one item, e.g. searching for plugins giving
    /// both `name` and `version`, or searching for anything giving `id`.
    pub async fn get_only(&self) -> Result<LinkedModel<R, A>, GetOnlyError> {
        if let Some(search) = &self.actual {
            search.get_only().await
        } else {
            Err(GetOnlyError::None)
        }
    }

    /// Produce items from this collection. Pagination is handled transparently,
    /// i.e. HTTP GET requests are sent as-needed.
    pub fn stream(&self) -> impl Stream<Item = Result<R, CubeError>> + '_ {
        stream! {
            let mut count = 0;
            let max_count = self.max_items.unwrap_or(usize::MAX);
            if let Some(search) = &self.actual {
                for await item in search.stream() {
                    if count >= max_count {
                        return;
                    }
                    yield item;
                    count += 1;
                }
            }
        }
    }

    /// Like [Self::stream], but clones the client for each item, so that methods can be called
    /// on the returned items.
    pub fn stream_connected(
        &self,
    ) -> impl Stream<Item = Result<LinkedModel<R, A>, CubeError>> + '_ {
        try_stream! {
            if let Some(search) = &self.actual {
                for await item in search.stream() {
                    yield LinkedModel { client: search.client.clone(), object: item?, phantom: Default::default() }
                }
            }
        }
    }
}

impl<R: DeserializeOwned> ActualSearch<R, RwAccess> {
    fn into_ro(self) -> ActualSearch<R, RoAccess> {
        ActualSearch {
            client: self.client,
            base_url: self.base_url,
            query: self.query,
            phantom: Default::default(),
            is_search: self.is_search,
        }
    }
}

impl<R: DeserializeOwned> Search<R, RwAccess> {
    /// Change yield type to generic of [RoAccess].
    pub fn into_ro(self) -> Search<R, RoAccess> {
        Search {
            actual: self.actual.map(|a| a.into_ro()),
            max_items: self.max_items,
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
    count: usize,
}
