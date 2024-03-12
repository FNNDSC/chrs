use crate::errors::CubeError;
use crate::search::Search;
use crate::types::PluginInstanceId;
use crate::{
    Access, LinkedModel, PipelineResponse, PluginInstanceResponse, RoAccess, RwAccess,
    WorkflowResponse,
};

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
    pub async fn create_workflow(
        &self,
        prev: PluginInstanceId,
        title: Option<&str>,
    ) -> Result<LinkedModel<WorkflowResponse, RwAccess>, CubeError> {
        let body = CreateWorkflow {
            previous_plugin_inst_id: prev,
            title,
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

pub type Workflow<A> = LinkedModel<WorkflowResponse, A>;

impl<A: Access> Workflow<A> {
    /// Get plugin instance of this workflow.
    pub fn plugin_instances(&self) -> Search<PluginInstanceResponse, A> {
        self.get_collection(&self.object.plugin_instances)
    }
}
