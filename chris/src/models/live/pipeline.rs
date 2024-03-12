use crate::search::Search;
use crate::{LinkedModel, PipelineResponse, RoAccess, RwAccess, WorkflowResponse};
use crate::errors::CubeError;
use crate::types::PluginInstanceId;

/// A _ChRIS_ pipeline.
pub type Pipeline<A> = LinkedModel<PipelineResponse, A>;

/// A _ChRIS_ pipeline, publicly-accessed.
pub type PipelineRo = LinkedModel<PipelineResponse, RoAccess>;

/// A _ChRIS_ pipeline you can run.
pub type PipelineRw = LinkedModel<PipelineResponse, RwAccess>;

impl PipelineRw {
    /// Get workflows (instances) of this pipeline.
    pub fn get_workflows(&self) -> Search<WorkflowResponse, RwAccess> {
        self.get_collection(&self.object.workflows)
    }

    /// Run this pipeline.
    pub async fn create_workflow(&self, prev: PluginInstanceId, title: Option<&str>) -> Result<LinkedModel<WorkflowResponse, RwAccess>, CubeError> {
        let body = CreateWorkflow {
            previous_plugin_inst_id: prev,
            title
        };
        self.post(&self.object.workflows, &body).await
    }
}

#[derive(serde::Serialize)]
struct CreateWorkflow<'a> {
    previous_plugin_inst_id: PluginInstanceId,
    title: Option<&'a str>,
    // nodes_info: idk
}