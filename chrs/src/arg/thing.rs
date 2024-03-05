use chris::types::*;

pub enum ChrsThing {
    PluginId(PluginId),
    PluginName(PluginName),
    PluginInstanceId(PluginInstanceId),
    PluginInstanceTitle(String),
    PipelineName(String),
    PipelineId(PipelineId),
    Ambiguous(String)
}
