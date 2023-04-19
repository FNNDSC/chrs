use crate::errors::{check, CUBEError};
use crate::models::data::{PipelineResponse, WorkflowCreatedResponse};
use crate::models::PluginInstanceId;
use reqwest::Client;
use serde::Serialize;

/// *ChRIS* pipeline.
pub struct Pipeline {
    client: Client,
    pub pipeline: PipelineResponse,
}

impl Pipeline {
    pub(crate) fn new(client: Client, res: PipelineResponse) -> Self {
        Self {
            client,
            pipeline: res,
        }
    }

    pub async fn create_workflow(
        &self,
        previous_plugin_inst_id: PluginInstanceId,
    ) -> Result<WorkflowCreatedResponse, CUBEError> {
        let payload = WorkflowPayload {
            previous_plugin_inst_id,
            nodes_info: "[]".to_string(),
        };
        let res = self
            .client
            .post(self.pipeline.workflows.as_str())
            .json(&payload)
            .send()
            .await?;
        Ok(check(res).await?.json().await?)
    }
}

#[derive(Serialize)]
struct WorkflowPayload {
    previous_plugin_inst_id: PluginInstanceId,
    nodes_info: String,
}
