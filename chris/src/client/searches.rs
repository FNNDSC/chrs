use crate::types::CollectionUrl;
use crate::Search;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Plugin search query
pub struct PluginSearchBuilder<'a, P: DeserializeOwned> {
    pub(crate) client: &'a reqwest_middleware::ClientWithMiddleware,
    pub(crate) url: &'a CollectionUrl,
    query: HashMap<&'static str, String>,
    phantom: PhantomData<P>,
}

impl<'a, P: DeserializeOwned> PluginSearchBuilder<'a, P> {
    /// Create a plugin search query
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

    /// Complete the plugin search query
    pub fn search(&self) -> Search<P, &HashMap<&'static str, String>> {
        Search::new(&self.client, &self.url, &self.query)
    }

    /// Search for plugin by name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.query.insert("name", name.into());
        self
    }

    /// Search for plugin by name_exact
    pub fn name_exact(mut self, name_exact: impl Into<String>) -> Self {
        self.query.insert("name_exact", name_exact.into());
        self
    }

    /// Search for plugin by version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.query.insert("version", version.into());
        self
    }
}
