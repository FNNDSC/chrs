//! Definitions of structs describing response data from the *CUBE* API.

use crate::types::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BaseResponse {
    /// Number of feeds. Is `None` if client is not logged in.
    pub count: Option<u32>,
    pub next: Option<CollectionUrl>,
    pub previous: Option<CollectionUrl>,
    pub collection_links: CubeLinks,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct CubeLinks {
    pub chrisinstance: ItemUrl,
    pub public_feeds: CollectionUrl,
    pub files: CollectionUrl,
    pub compute_resources: CollectionUrl,
    pub plugin_metas: CollectionUrl,
    pub plugins: CollectionUrl,
    pub plugin_instances: CollectionUrl,
    pub pipelines: CollectionUrl,
    pub pipeline_instances: CollectionUrl,
    pub workflows: CollectionUrl,
    pub tags: CollectionUrl,
    pub pipelinesourcefiles: CollectionUrl,
    pub pacsfiles: CollectionUrl,
    pub servicefiles: CollectionUrl,
    pub filebrowser: FileBrowserUrl,

    // Renamed in https://github.com/FNNDSC/ChRIS_ultron_backEnd/pull/528
    #[serde(alias = "userfiles", alias = "uploadedfiles")]
    pub userfiles: Option<CollectionUrl>,

    pub user: Option<ItemUrl>,
    pub admin: Option<CollectionUrl>,
}

#[derive(Debug, Deserialize)]
pub struct PipelineResponse {
    pub url: ItemUrl,
    pub id: PipelineId,
    pub name: String,
    pub locked: bool,
    pub authors: String,
    pub category: String,
    pub description: String,
    pub owner_username: Username,
    pub creation_date: DateString,
    pub modification_date: DateString,
    pub plugins: CollectionUrl,
    pub plugin_pipings: CollectionUrl,
    pub default_parameters: CollectionUrl,
    pub instances: CollectionUrl,
    pub workflows: CollectionUrl,
}

#[derive(Debug, Deserialize)]
pub struct PluginResponse {
    pub url: PluginUrl,
    pub id: PluginId,
    pub creation_date: String,
    pub name: PluginName,
    pub version: PluginVersion,
    pub dock_image: DockImage,
    pub public_repo: PluginRepo,
    pub icon: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    pub stars: u32,
    pub authors: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub documentation: String,
    pub license: String,
    pub execshell: String,
    pub selfpath: String,
    pub selfexec: String,
    pub min_number_of_workers: u32,
    pub max_number_of_workers: u32,
    pub min_cpu_limit: u32,
    pub max_cpu_limit: u32,
    pub min_memory_limit: u32,
    pub max_memory_limit: u32,
    pub min_gpu_limit: u32,
    pub max_gpu_limit: u32,
    pub meta: ItemUrl,
    pub parameters: CollectionUrl,
    pub instances: CollectionUrl,
    pub compute_resources: CollectionUrl,
}

#[derive(Deserialize)]
pub struct FeedResponse {
    pub url: ItemUrl,
    pub name: String,
    pub creator_username: Username,
    pub id: FeedId,
    pub creation_date: DateString, // many fields missing ;-;
}

#[derive(Deserialize, Debug)]
pub struct PluginInstanceResponse {
    pub url: ItemUrl,
    pub id: PluginInstanceId,
    pub title: String,
    /// N.B.: compute_resource might be null if the compute resource
    /// was deleted.
    pub compute_resource: Option<ItemUrl>,
    pub compute_resource_name: Option<ComputeResourceName>,
    pub plugin: PluginUrl,
    pub plugin_id: PluginId,
    pub plugin_name: PluginName,
    pub plugin_version: PluginVersion,
    pub plugin_type: PluginType,
    // pipeline_inst: Option<String>,
    pub start_date: String,
    pub end_date: String,
    pub output_path: String,
    pub status: String,
    pub summary: String,
    pub raw: String,
    pub owner_username: Username,
    pub cpu_limit: u32,
    pub memory_limit: u32,
    pub number_of_workers: u32,
    pub gpu_limit: u32,
    pub size: u64,
    pub error_code: String,
    pub previous: Option<ItemUrl>,
    pub feed: ItemUrl,
    pub descendants: CollectionUrl,
    pub files: CollectionUrl,
    pub parameters: CollectionUrl,
    pub splits: CollectionUrl,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct PluginParameter {
    pub url: ItemUrl,
    pub id: PluginParameterId,
    pub name: String,
    #[serde(rename = "type")]
    pub parameter_type: PluginParameterType,
    pub optional: bool,
    pub default: Option<PluginParameterValue>,
    pub flag: String,
    pub short_flag: String,
    pub action: PluginParameterAction,
    pub help: String,
    pub ui_exposed: bool,
    pub plugin: PluginUrl,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowCreatedResponse {
    pub url: ItemUrl,
    pub id: WorkflowId,
    pub creation_date: String,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub owner_username: Username,
    pub pipeline: ItemUrl,
}
