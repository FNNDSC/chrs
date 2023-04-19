//! Definitions of structs describing response data from the *CUBE* API.

use super::enums::*;
use super::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct BaseResponse {
    pub count: u32,
    pub next: Option<FeedsPaginatedUrl>,
    pub previous: Option<FeedsPaginatedUrl>,
    pub collection_links: CubeLinks,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CubeLinks {
    pub files: FeedFilesUrl,
    pub uploadedfiles: UploadedFilesUrl,
    pub user: UserUrl,
    pub pipelines: PipelinesUrl,
    pub filebrowser: FileBrowserUrl,
    pub plugins: PluginsUrl,
    pub plugin_instances: PluginInstancesUrl,
    pub pacsfiles: PacsFilesUrl,
    pub servicefiles: ServiceFilesUrl,
}

#[derive(Deserialize)]
pub struct DownloadableFile {
    pub(crate) file_resource: FileResourceUrl,
    pub(crate) fname: FileResourceFname,
    pub(crate) fsize: u64,
}

#[derive(Debug, Deserialize)]
pub struct PipelineResponse {
    pub url: PipelineUrl,
    pub id: PipelineId,
    pub name: String,
    pub locked: bool,
    pub authors: String,
    pub category: String,
    pub description: String,
    pub owner_username: Username,
    pub creation_date: String,
    pub modification_date: String,
    pub plugins: PipelinePluginsUrl,
    pub plugin_pipings: PipelinePipingsUrl,
    pub default_parameters: PipelineDefaultParametersUrl,
    pub instances: PipelineInstancesUrl,
    pub workflows: PipelineWorkflowsUrl,
}

#[derive(Debug, Deserialize)]
pub struct FileUploadResponse {
    pub url: String,
    pub id: u32,
    pub creation_date: String,
    pub(crate) fname: FileResourceFname,
    pub(crate) fsize: u64,
    pub(crate) file_resource: FileResourceUrl,
    pub owner: String,
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
    pub meta: PluginMetaUrl,
    pub parameters: PluginParametersUrl,
    pub instances: PluginInstancesUrl,
    pub compute_resources: PluginComputeResourcesUrl,
}

#[derive(Deserialize)]
pub struct FeedResponse {
    pub url: FeedUrl,
    pub name: String,
    pub creator_username: Username,
    pub id: FeedId,
    // pub creation_date:
    // many fields missing ;-;
}

#[derive(Deserialize, Debug)]
pub struct PluginInstanceResponse {
    pub url: PluginInstanceUrl,
    pub id: PluginInstanceId,
    pub title: String,
    /// N.B.: compute_resource might be null if the compute resource
    /// was deleted.
    pub compute_resource: Option<ComputeResourceUrl>,
    pub compute_resource_name: Option<ComputeResourceName>,
    pub plugin: PluginUrl,
    pub plugin_id: PluginId,
    pub plugin_name: PluginName,
    pub plugin_version: PluginVersion,
    pub plugin_type: PluginType,
    // pipeline_inst: Option<String>,  // TODO
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
    pub previous: Option<PluginInstanceUrl>,
    pub feed: FeedUrl,
    pub descendants: DescendantsUrl,
    pub files: PluginInstanceFilesUrl,
    pub parameters: PluginInstanceParametersUrl,
    pub splits: PluginInstanceSplitsUrl,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct PluginParameter {
    pub url: PluginParameterUrl,
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
    pub url: WorkflowUrl,
    pub id: WorkflowId,
    pub creation_date: String,
    pub pipeline_id: PipelineId,
    pub pipeline_name: String,
    pub owner_username: Username,
    pub pipeline: PipelineUrl,
}
