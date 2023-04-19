use serde::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

pub use super::auth_types::*;
use aliri_braid::braid;

#[braid(serde)]
pub struct FeedsPaginatedUrl;

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

/// CUBE PACS files URL, e.g. `https://cube.chrisproject.org/api/v1/pacsfiles/`
#[braid(serde)]
pub struct PacsFilesUrl;

/// CUBE services files API URL, e.g. `https://cube.chrisproject.org/api/v1/servicefiles/`
#[braid(serde)]
pub struct ServiceFilesUrl;

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

#[braid(serde)]
pub struct PluginInstanceUrl;

#[braid(serde)]
pub struct FeedUrl;

#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct FeedId(pub u32);

#[derive(Copy, Clone, Shrinkwrap, Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct PluginInstanceId(pub u32);

#[braid(serde)]
pub struct DescendantsUrl;

#[braid(serde)]
pub struct PluginInstanceParametersUrl;

#[braid(serde)]
pub struct PluginInstanceSplitsUrl;

#[braid(serde)]
pub struct PluginInstanceFilesUrl;

#[braid(serde)]
pub struct ComputeResourceUrl;

#[braid(serde)]
pub struct ComputeResourceName;

#[braid(serde)]
pub struct PluginParameterUrl;

#[derive(Shrinkwrap, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub struct PluginParameterId(pub u32);

#[derive(Shrinkwrap, Deserialize, Debug)]
pub struct WorkflowId(pub u32);

#[braid(serde)]
pub struct WorkflowUrl;
