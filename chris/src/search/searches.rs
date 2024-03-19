use crate::types::{
    FeedId, PacsFileId, PipelineId, PluginId, PluginInstanceId, Username, WorkflowId,
};
use crate::{
    Access, FeedFileResponse, FeedResponse, PacsFileResponse, PipelineResponse,
    PluginInstanceResponse, PluginResponse, WorkflowResponse,
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

/// PACSFiles search query
pub type PacsFilesSearchBuilder<A> = QueryBuilder<PacsFileResponse, A>;

impl<A: Access> PacsFilesSearchBuilder<A> {
    /// Search for PACSFile by ID
    pub fn id(self, id: PacsFileId) -> Self {
        self.add_u32("id", id.0)
    }
    pub fn fname(self, fname: impl Into<String>) -> Self {
        self.add_string("fname", fname)
    }
    pub fn fname_exact(self, fname_exact: impl Into<String>) -> Self {
        self.add_string("fname_exact", fname_exact)
    }
    pub fn fname_icontains(self, fname_icontains: impl Into<String>) -> Self {
        self.add_string("fname_icontains", fname_icontains)
    }
    pub fn fname_icontains_topdir_unique(
        self,
        fname_icontains_topdir_unique: impl Into<String>,
    ) -> Self {
        self.add_string(
            "fname_icontains_topdir_unique",
            fname_icontains_topdir_unique,
        )
    }
    pub fn fname_nslashes(self, fname_nslashes: u32) -> Self {
        self.add_u32("fname_nslashes", fname_nslashes)
    }
    pub fn patient_id(self, patient_id: impl Into<String>) -> Self {
        self.add_string("PatientID", patient_id)
    }
    pub fn patient_name(self, patient_name: impl Into<String>) -> Self {
        self.add_string("PatientName", patient_name)
    }
    pub fn patient_sex(self, patient_sex: impl Into<String>) -> Self {
        self.add_string("PatientSex", patient_sex)
    }
    pub fn patient_age(self, age_in_days: u32) -> Self {
        // maybe accept duration instead?
        self.add_u32("PatientAge", age_in_days)
    }
    pub fn min_patient_age(self, age_in_days: u32) -> Self {
        self.add_u32("min_PatientAge", age_in_days)
    }
    pub fn max_patient_age(self, age_in_days: u32) -> Self {
        self.add_u32("max_PatientAge", age_in_days)
    }
    pub fn patient_birth_date(self, patient_birth_date: impl Into<String>) -> Self {
        self.add_string("PatientBirthDate", patient_birth_date)
    }
    pub fn study_date(self, study_date: impl Into<String>) -> Self {
        self.add_string("StudyDate", study_date)
    }
    pub fn accession_number(self, accession_number: impl Into<String>) -> Self {
        self.add_string("AccessionNumber", accession_number)
    }
    pub fn protocol_name(self, protocol_name: impl Into<String>) -> Self {
        self.add_string("ProtocolName", protocol_name)
    }
    pub fn study_instance_uid(self, study_instance_uid: impl Into<String>) -> Self {
        self.add_string("StudyInstanceUID", study_instance_uid)
    }
    pub fn study_description(self, study_description: impl Into<String>) -> Self {
        self.add_string("StudyDescription", study_description)
    }
    pub fn series_instance_uid(self, series_instance_uid: impl Into<String>) -> Self {
        self.add_string("SeriesInstanceUID", series_instance_uid)
    }
    pub fn series_description(self, series_description: impl Into<String>) -> Self {
        self.add_string("SeriesDescription", series_description)
    }
    pub fn pacs_identifier(self, pacs_identifier: impl Into<String>) -> Self {
        self.add_string("pacs_identifier", pacs_identifier)
    }
}
