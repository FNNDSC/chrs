//! Types produced by the _ChRIS_ backend (CUBE) API.

use crate::common_types::Username;
use aliri_braid::braid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct BaseResponse {
    pub count: u32,
    pub next: Option<FeedsPaginatedUrl>,
    pub previous: Option<FeedsPaginatedUrl>,
    pub collection_links: CUBELinks,
}

#[braid(serde)]
pub struct FeedsPaginatedUrl;

#[derive(Debug, Deserialize)]
pub struct CUBELinks {
    pub files: FeedFilesUrl,
    pub uploadedfiles: UploadedFilesUrl,
    pub user: UserUrl,
    pub pipelines: PipelinesUrl,
    pub filebrowser: FileBrowserUrl,
    pub plugins: PluginsUrl,
    pub plugin_instances: PluginInstancesUrl,
}

/// CUBE file browser API URL, e.g. `https://cube.chrisproject.org/api/v1/filebrowser/`
#[braid(serde)]
pub struct FileBrowserUrl;

/// CUBE files resource URL, e.g. `https://cube.chrisproject.org/api/v1/files/`
#[braid(serde)]
pub struct FeedFilesUrl;

/// CUBE uploaded files resource URL, e.g. `https://cube.chrisproject.org/api/v1/uploadedfiles/`
#[braid(serde)]
pub struct UploadedFilesUrl;

/// CUBE plugins resource URL, e.g. `https://cube.chrisproject.org/api/v1/plugins/`
#[braid(serde)]
pub struct PluginsUrl;

/// CUBE User ID
#[derive(Shrinkwrap, Deserialize)]
pub struct UserId(pub u32);

/// CUBE user resource URL, e.g. `https://cube.chrisproject.org/api/v1/users/3/`
#[braid(serde)]
pub struct UserUrl;

/// CUBE pipelines resource URL, e.g. `https://cube.chrisproject.org/api/v1/pipelines/`
#[braid(serde)]
pub struct PipelinesUrl;

#[braid(serde)]
pub struct PluginName;

#[braid(serde)]
pub struct PluginVersion;

#[braid(serde)]
pub struct ParameterName;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum ParameterValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

#[braid(serde)]
pub struct PipelineUrl;

#[derive(Shrinkwrap, Deserialize, Debug)]
pub struct PipelineId(pub u32);

#[braid(serde)]
pub struct PipelinePluginsUrl;

#[braid(serde)]
pub struct PipelinePipingsUrl;

#[braid(serde)]
pub struct PipelineDefaultParametersUrl;

#[braid(serde)]
pub struct PipelineInstancesUrl;

#[braid(serde)]
pub struct PipelineWorkflowsUrl;

/// A URL which produces a collection of files.
///
/// # Examples
///
/// - `https://cube.chrisproject.org/api/v1/files/`
/// - `https://cube.chrisproject.org/api/v1/files/search/`
/// - `https://cube.chrisproject.org/api/v1/uploadedfiles/search/?fname=txt`
/// - `https://cube.chrisproject.org/api/v1/20/files/`
/// - `https://cube.chrisproject.org/api/v1/plugins/instances/40/files/`
#[braid(serde)]
pub struct AnyFilesUrl;

/// Download URL for a file resource.
///
/// # Examples
///
/// - `https://cube.chrisproject.org/api/v1/files/84360/aparc.a2009s+aseg.mgz`
#[braid(serde)]
pub struct FileResourceUrl;

/// File fname.
#[braid(serde)]
pub struct FileResourceFname;

/// A CUBE item which has a `file_resource` and `fname`.
pub trait Downloadable {
    fn file_resource(&self) -> &FileResourceUrl;
    fn fname(&self) -> &FileResourceFname;
    fn fsize(&self) -> u64;
}

#[derive(Deserialize)]
pub struct DownloadableFile {
    file_resource: FileResourceUrl,
    fname: FileResourceFname,
    fsize: u64,
}

impl Downloadable for DownloadableFile {
    fn file_resource(&self) -> &FileResourceUrl {
        &self.file_resource
    }

    fn fname(&self) -> &FileResourceFname {
        &self.fname
    }

    fn fsize(&self) -> u64 {
        self.fsize
    }
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
    fname: FileResourceFname,
    fsize: u64,
    file_resource: FileResourceUrl,
    pub owner: String,
}

impl Downloadable for FileUploadResponse {
    fn file_resource(&self) -> &FileResourceUrl {
        &self.file_resource
    }

    fn fname(&self) -> &FileResourceFname {
        &self.fname
    }

    fn fsize(&self) -> u64 {
        self.fsize
    }
}

/// Plugin URL.
#[braid(serde)]
pub struct PluginUrl;

/// Plugin ID
#[derive(Shrinkwrap, Deserialize, Debug)]
pub struct PluginId(pub u32);

/// Container image name of a plugin.
#[braid(serde)]
pub struct DockImage;

/// Public source code repository of a plugin.
#[braid(serde)]
pub struct PluginRepo;

/// <https://github.com/FNNDSC/CHRIS_docs/blob/master/specs/ChRIS_Plugins.adoc#plugin-type>
#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Fs,
    Ds,
    Ts,
}

/// Plugin meta URL.
#[braid(serde)]
pub struct PluginMetaUrl;

/// Plugin parameters URL.
#[braid(serde)]
pub struct PluginParametersUrl;

/// Plugin instances URL.
#[braid(serde)]
pub struct PluginInstancesUrl;

/// Plugin compute resources URL.
#[braid(serde)]
pub struct PluginComputeResourcesUrl;

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

#[braid(serde)]
pub struct PluginInstanceUrl;

#[braid(serde)]
pub struct FeedUrl;

#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PluginInstanceId(pub u32);

#[braid(serde)]
pub struct DescendantsUrl;

#[braid(serde)]
pub struct PluginInstanceParametersUrl;

#[braid(serde)]
pub struct PluginInstanceSplitsUrl;

#[braid(serde)]
pub struct ComputeResourceUrl;

#[braid(serde)]
pub struct ComputeResourceName;

#[derive(Deserialize, Debug)]
pub struct PluginInstanceCreatedResponse {
    pub url: PluginInstanceUrl,
    pub id: PluginInstanceId,
    pub title: String,
    pub compute_resource_name: ComputeResourceName,
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
    pub plugin: PluginUrl,
    pub descendants: DescendantsUrl,
    pub files: AnyFilesUrl,
    pub parameters: PluginInstanceParametersUrl,
    pub compute_resource: ComputeResourceUrl,
    pub splits: PluginInstanceSplitsUrl,
}

#[braid(serde)]
pub struct PluginParameterUrl;

#[derive(Shrinkwrap, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct PluginParameterId(pub u32);

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

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginParameterType {
    Boolean,
    Integer,
    Float,
    String,
    Path,
    Unextpath,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum PluginParameterValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),

    /// Either a `str`, `path`, or `unextpath`
    Stringish(String),
}

#[derive(Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum PluginParameterAction {
    #[serde(rename = "store")]
    Store,
    #[serde(rename = "store_true")]
    StoreTrue,
    #[serde(rename = "store_false")]
    StoreFalse,
}

#[derive(Shrinkwrap, Deserialize, Debug)]
pub struct WorkflowId(pub u32);

#[braid(serde)]
pub struct WorkflowUrl;

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
