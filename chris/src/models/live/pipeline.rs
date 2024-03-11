use crate::search::Search;
use crate::{LinkedModel, PipelineResponse, RoAccess, RwAccess, WorkflowResponse};

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
}
