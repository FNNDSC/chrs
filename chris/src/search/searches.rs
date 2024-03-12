use crate::types::{FeedId, PipelineId, PluginId, PluginInstanceId, Username, WorkflowId};
use crate::{
    Access, FeedFileResponse, FeedResponse, PipelineResponse, PluginInstanceResponse,
    PluginResponse, WorkflowResponse,
};

use super::query::QueryBuilder;

/// Plugin search query
pub type PluginSearchBuilder<A> = QueryBuilder<PluginResponse, A>;

impl<A: Access> PluginSearchBuilder<A> {
    /// Search for plugin by ID
    pub fn id(self, id: PluginId) -> Self {
        self.add_u32("id", id.0)
    }

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

    /// Search by plugin by name, title, or category
    pub fn name_title_category(self, name_title_category: impl Into<String>) -> Self {
        self.add_string("name_title_category", name_title_category)
    }
}

/// Plugin search query
pub type FeedSearchBuilder<A> = QueryBuilder<FeedResponse, A>;

impl<A: Access> FeedSearchBuilder<A> {
    /// Search for feed by name
    pub fn name(self, name: impl Into<String>) -> Self {
        self.add_string("name", name)
    }

    /// Search for feed by name_exact
    pub fn name_exact(self, name_exact: impl Into<String>) -> Self {
        self.add_string("name_exact", name_exact)
    }
}

/// Plugin instance search query
pub type PluginInstanceSearchBuilder<A> = QueryBuilder<PluginInstanceResponse, A>;

impl<A: Access> PluginInstanceSearchBuilder<A> {
    /// Search for plugin instance by ID
    pub fn id(self, id: PluginInstanceId) -> Self {
        self.add_u32("id", id.0)
    }

    /// Search for plugin instance by the ID of its previous
    pub fn previous_id(self, previous_id: PluginInstanceId) -> Self {
        self.add_u32("previous_id", previous_id.0)
    }

    /// Search for plugin instance by title
    pub fn title(self, title: impl Into<String>) -> Self {
        self.add_string("title", title)
    }

    /// Search for plugin instance by feed_id
    pub fn feed_id(self, feed_id: FeedId) -> Self {
        self.add_u32("feed_id", feed_id.0)
    }

    /// Search for plugin instance by plugin_name
    pub fn plugin_name(self, plugin_name: impl Into<String>) -> Self {
        self.add_string("plugin_name", plugin_name)
    }

    /// Search for plugin instance by plugin_name_exact
    pub fn plugin_name_exact(self, plugin_name_exact: impl Into<String>) -> Self {
        self.add_string("plugin_name_exact", plugin_name_exact)
    }

    /// Search for plugin instance by plugin_version
    pub fn plugin_version(self, plugin_version: impl Into<String>) -> Self {
        self.add_string("plugin_version", plugin_version)
    }

    /// Search for plugin instance by workflow_id
    pub fn workflow_id(self, workflow_id: WorkflowId) -> Self {
        self.add_u32("workflow_id", workflow_id.0)
    }
}

/// Pipeline search query
pub type PipelineSearchBuilder<A> = QueryBuilder<PipelineResponse, A>;

impl<A: Access> PipelineSearchBuilder<A> {
    /// Search for pipeline by ID
    pub fn id(self, id: PipelineId) -> Self {
        self.add_u32("id", id.0)
    }

    /// Search for pipeline by name
    pub fn name(self, name: impl Into<String>) -> Self {
        self.add_string("name", name)
    }

    /// Search for pipeline by description
    pub fn description(self, description: impl Into<String>) -> Self {
        self.add_string("description", description)
    }
}

/// File search query. Only searches for files produced by plugin instances.
pub type FilesSearchBuilder<A> = QueryBuilder<FeedFileResponse, A>;

impl<A: Access> FilesSearchBuilder<A> {
    /// Search for files by plugin instance ID
    pub fn plugin_inst_id(self, plugin_inst_id: PluginInstanceId) -> Self {
        self.add_u32("plugin_inst_id", plugin_inst_id.0)
    }

    /// Search for files by feed ID
    pub fn feed_id(self, feed_id: FeedId) -> Self {
        self.add_u32("feed_id", feed_id.0)
    }

    /// Search for files by fname (starts with)
    pub fn fname(self, fname: impl Into<String>) -> Self {
        self.add_string("fname", fname)
    }

    /// Search for files by fname (exact match)
    pub fn fname_exact(self, fname_exact: impl Into<String>) -> Self {
        self.add_string("fname_exact", fname_exact)
    }

    /// Search for files by fname (contains case-insensitive)
    pub fn fname_icontains(self, fname_icontains: impl Into<String>) -> Self {
        self.add_string("fname_icontains", fname_icontains)
    }

    /// Search for files by number of slashes in fname.
    pub fn fname_nslashes(self, fname_nslashes: u32) -> Self {
        self.add_u32("fname_nslashes", fname_nslashes)
    }
}

/// Workflow search query
pub type WorkflowSearchBuilder<A> = QueryBuilder<WorkflowResponse, A>;

impl<A: Access> WorkflowSearchBuilder<A> {
    /// Search for workflow by ID
    pub fn id(self, id: WorkflowId) -> Self {
        self.add_u32("id", id.0)
    }

    /// Search for workflow by title
    pub fn title(self, title: impl Into<String>) -> Self {
        self.add_string("title", title)
    }

    /// Search for workflow by pipeline name
    pub fn pipeline_name(self, pipeline_name: impl Into<String>) -> Self {
        self.add_string("pipeline_name", pipeline_name)
    }

    /// Search for workflow by owner_username
    pub fn owner_username(self, owner_username: &Username) -> Self {
        self.add_string("owner_username", owner_username.as_str())
    }
}
