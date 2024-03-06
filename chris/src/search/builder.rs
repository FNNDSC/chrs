use super::Search;
use crate::types::CollectionUrl;
use crate::Access;
use reqwest_middleware::ClientWithMiddleware;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Serialize)]
#[serde(untagged)]
pub enum QueryValue {
    U32(u32),
    String(String),
}

/// A `SearchBuilder` builds a request for a search API, e.g. `api/v1/plugins/search/`,
/// or a request to a collection API, e.g. `api/v1/plugins/`.
pub struct SearchBuilder<'a, T: DeserializeOwned, A: Access> {
    pub(crate) client: &'a ClientWithMiddleware,
    pub(crate) url: &'a CollectionUrl,
    query: HashMap<&'static str, QueryValue>,
    phantom: PhantomData<(A, T)>,
    max_items: Option<usize>,
    is_search: bool,
}

impl<'a, T: DeserializeOwned, A: Access> SearchBuilder<'a, T, A> {
    /// Create a search query
    pub(crate) fn query(client: &'a ClientWithMiddleware, url: &'a CollectionUrl) -> Self {
        Self {
            client,
            url,
            query: Default::default(),
            phantom: Default::default(),
            max_items: None,
            is_search: true,
        }
    }

    /// Create a request to fetch a collection (without search query terms).
    pub(crate) fn collection(client: &'a ClientWithMiddleware, url: &'a CollectionUrl) -> Self {
        Self {
            client,
            url,
            query: Default::default(),
            phantom: Default::default(),
            max_items: None,
            is_search: false,
        }
    }

    /// Complete the search query
    pub fn search(&self) -> Search<T, A, &HashMap<&'static str, QueryValue>> {
        if self.is_search {
            Search::search(self.client, self.url, &self.query, self.max_items)
        } else {
            Search::collection(self.client, self.url, &self.query, self.max_items)
        }
    }

    /// Set maximum number of items to return per page. The only reason to set this would
    /// be for performance reasons. Generally you don't need to set it.
    ///
    /// See also: [Self::max_items]
    pub fn page_limit(self, limit: u32) -> Self {
        self.add_u32("limit", limit)
    }

    /// Caps the number of items to produce.
    pub fn max_items(self, max_items: usize) -> Self {
        Self {
            client: self.client,
            url: self.url,
            query: self.query,
            phantom: Default::default(),
            max_items: Some(max_items),
            is_search: self.is_search,
        }
    }

    pub(crate) fn add_string(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.query.insert(key, QueryValue::String(value.into()));
        self
    }

    pub(crate) fn add_u32(mut self, key: &'static str, value: u32) -> Self {
        self.query.insert(key, QueryValue::U32(value));
        self
    }
}
