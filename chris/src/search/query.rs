use std::collections::HashMap;
use std::marker::PhantomData;

use reqwest_middleware::ClientWithMiddleware;
use serde::de::DeserializeOwned;

use crate::types::CollectionUrl;
use crate::{Access, RoAccess, RwAccess};

use super::{QueryValue, Search};

/// A `SearchBuilder` builds a request for a search API, e.g. `api/v1/plugins/search/`,
/// or a request to a collection API, e.g. `api/v1/plugins/`.
#[derive(Clone)]
pub struct QueryBuilder<T: DeserializeOwned, A: Access> {
    pub(crate) client: ClientWithMiddleware,
    pub(crate) url: CollectionUrl,
    query: HashMap<&'static str, QueryValue>,
    phantom: PhantomData<(A, T)>,
    page_limit: Option<u32>,
    max_items: Option<usize>,
}

impl<T: DeserializeOwned> QueryBuilder<T, RwAccess> {
    /// Convert this [QueryBuilder] to produce [RoAccess] items.
    pub fn into_ro(self) -> QueryBuilder<T, RoAccess> {
        QueryBuilder {
            client: self.client,
            url: self.url,
            query: self.query,
            page_limit: self.page_limit,
            max_items: self.max_items,
            phantom: Default::default(),
        }
    }
}

impl<T: DeserializeOwned, A: Access> QueryBuilder<T, A> {
    /// Create a search query
    pub(crate) fn query(client: ClientWithMiddleware, url: CollectionUrl) -> Self {
        Self {
            client,
            url,
            query: Default::default(),
            phantom: Default::default(),
            page_limit: None,
            max_items: None,
        }
    }

    /// Complete the search query
    pub fn search(self) -> Search<T, A> {
        Search::with_query(self.client, self.url, self.query)
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
