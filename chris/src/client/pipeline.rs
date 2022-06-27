use reqwest::Client;
use crate::api::{PipelineResponse, PluginInstanceId};

pub struct Pipeline {
    client: Client,
    pub pipeline: PipelineResponse,
}

impl Pipeline {
    pub(crate) fn new(client: Client, res: PipelineResponse) -> Self {
        Self { client, pipeline: res }
    }

    pub fn create_workflow(previous_plugin_inst_id: PluginInstanceId) {
        
    }
}
