//! Helpers for pagination.

use crate::errors::{check, CUBEError};
use crate::models::ConnectedModel;
use async_stream::try_stream;
use futures::Stream;
use reqwest::{RequestBuilder, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// An abstraction over collection APIs, i.e. paginated API endpoints which return a `results` list.
///
/// This is homologus to the Python implementation in aiochris:
///
/// https://github.com/FNNDSC/aiochris/blob/adaff5bbc1d4d886ec2ca8155d82d266fa81d093/chris/util/search.py
pub struct Search<R: DeserializeOwned, Q: Serialize + Sized> {
    client: reqwest::Client,
    base_url: String,
    query: Q,
    phantom: PhantomData<R>,
}

impl<R: DeserializeOwned, Q: Serialize + Sized> Search<R, Q> {
    pub(crate) fn new(client: &reqwest::Client, base_url: impl ToString, query: Q) -> Self {
        Self {
            client: client.clone(),
            base_url: base_url.to_string(),
            query,
            phantom: Default::default(),
        }
    }

    /// Get the count of items in this collection.
    pub async fn get_count(&self) -> Result<u32, CUBEError> {
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

impl<R: DeserializeOwned, Q: Serialize + Sized> Search<R, Q> {
    fn get_search(&self) -> RequestBuilder {
        let url = format!("{}search/", &self.base_url);
        self.client.get(url).query(&self.query)
    }

    /// Get the first item from this collection.
    ///
    /// See also: [Search::get_only]
    pub async fn get_first(&self) -> Result<Option<ConnectedModel<R>>, CUBEError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let page: Paginated<R> = check(res).await?.json().await?;
        let first = page.results.into_iter().next();
        let ret = first.map(|data| ConnectedModel {
            client: self.client.clone(),
            data,
        });
        Ok(ret)
    }

    /// Get the _only_ item from this collection.
    ///
    /// This function _should_ only be called when some invariant holds that
    /// the collection has only one item, e.g. searching for plugins giving
    /// both `name` and `version`, or searching for anything giving `id`.
    pub async fn get_only(&self) -> Result<ConnectedModel<R>, GetOnlyError> {
        let res = self.get_search().query(&LIMIT_ONE).send().await?;
        let page: Paginated<R> = check(res).await?.json().await?;

        if page.count > 1 {
            return Err(GetOnlyError::MoreThanOne);
        }

        if let Some(data) = page.results.into_iter().next() {
            Ok(ConnectedModel {
                client: self.client.clone(),
                data,
            })
        } else {
            Err(GetOnlyError::None)
        }
    }

    /// Produce items from this collection. Pagination is handled transparently,
    /// i.e. HTTP GET requests are sent as-needed.
    pub fn stream(&self) -> impl Stream<Item = Result<R, CUBEError>> + '_ {
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

    /// Like [Self::stream], but clones the client for each item, so that methods can be called
    /// on the returned items.
    pub fn stream_connected(
        &self,
    ) -> impl Stream<Item = Result<ConnectedModel<R>, CUBEError>> + '_ {
        try_stream! {
            for await item in self.stream() {
                yield ConnectedModel { client: self.client.clone(), data: item? }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Paginated<R> {
    pub count: u32,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<R>,
}

#[derive(thiserror::Error, Debug)]
pub enum GetOnlyError {
    #[error("Empty collection")]
    None,
    #[error("More than one result in collection")]
    MoreThanOne,
    #[error(transparent)]
    Error(#[from] CUBEError),
}

impl From<reqwest::Error> for GetOnlyError {
    fn from(value: reqwest::Error) -> Self {
        CUBEError::Raw(value).into()
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

#[derive(Deserialize)]
struct HasCount {
    count: u32,
}
