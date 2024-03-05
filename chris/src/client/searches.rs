use super::variant::Access;
use crate::types::CollectionUrl;
use crate::{FeedResponse, PluginResponse, Search};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Serialize)]
#[serde(untagged)]
pub enum QueryValue {
    U32(u32),
    String(String),
}

/// A `SearchBuilder` builds a request for a search API, e.g. `api/v1/plugins/search/`
pub struct SearchBuilder<'a, A: Access, T: DeserializeOwned> {
    pub(crate) client: &'a reqwest_middleware::ClientWithMiddleware,
    pub(crate) url: &'a CollectionUrl,
    query: HashMap<&'static str, QueryValue>,
    phantom: PhantomData<(A, T)>,
}

impl<'a, A: Access, T: DeserializeOwned> SearchBuilder<'a, A, T> {
    /// Create a search query
    pub(crate) fn new(
        client: &'a reqwest_middleware::ClientWithMiddleware,
        url: &'a CollectionUrl,
    ) -> Self {
        Self {
            client,
            url,
            query: Default::default(),
            phantom: Default::default(),
        }
    }

    /// Complete the search query
    pub fn search(&self) -> Search<T, A, &HashMap<&'static str, QueryValue>> {
        Search::new(self.client, self.url, &self.query)
    }

    /// Set maximum number of items to return per page. The only reason to set this would
    /// be for performance reasons. Generally you don't need to set it.
    pub fn limit(self, limit: u32) -> Self {
        self.add_u32("limit", limit)
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

/// Plugin search query
pub type PluginSearchBuilder<'a, A> = SearchBuilder<'a, A, PluginResponse>;

impl<A: Access> PluginSearchBuilder<'_, A> {
    /// Search for plugin by name
    pub fn name(self, name: impl Into<String>) -> Self {
        self.add_string("name", name)
    }

    /// Search for plugin by name_exact
    pub fn name_exact(self, name_exact: impl Into<String>) -> Self {
        self.add_string("name_exact", name_exact)
    }

    /// Search for plugin by version
    pub fn version(self, version: impl Into<String>) -> Self {
        self.add_string("version", version)
    }
}

/// Plugin search query
pub type FeedSearchBuilder<'a, A> = SearchBuilder<'a, A, FeedResponse>;

impl<A: Access> FeedSearchBuilder<'_, A> {
    /// Search for feed by name
    pub fn name(self, name: impl Into<String>) -> Self {
        self.add_string("name", name)
    }

    /// Search for feed by name_exact
    pub fn name_exact(self, name_exact: impl Into<String>) -> Self {
        self.add_string("name_exact", name_exact)
    }
}
