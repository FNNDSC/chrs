use aliri_braid::braid;
use serde::{Deserialize, Serialize};
use crate::common_types::Username;

#[derive(Debug, Deserialize)]
pub struct BaseResponse {
    pub count: u32,
    pub next: Option<FeedsPaginatedUrl>,
    pub previous: Option<FeedsPaginatedUrl>,
    pub collection_links: CUBELinks
}

#[braid(serde)]
pub struct FeedsPaginatedUrl;

#[derive(Debug, Deserialize)]
pub struct CUBELinks {
    pub files: FilesUrl,
    pub uploadedfiles: UploadedFilesUrl,
    pub user: UserUrl,
    pub pipelines: PipelinesUrl,
}

/// CUBE files resource URL, e.g. https://cube.chrisproject.org/api/v1/files/
#[braid(serde)]
pub struct FilesUrl;

/// CUBE uploaded files resource URL, e.g. https://cube.chrisproject.org/api/v1/uploadedfiles/
#[braid(serde)]
pub struct UploadedFilesUrl;

/// CUBE User ID
#[derive(Shrinkwrap, Deserialize)]
pub struct UserId(u16);

/// CUBE user resource URL, e.g. https://cube.chrisproject.org/api/v1/users/3/
#[braid(serde)]
pub struct UserUrl;

/// CUBE pipelines resource URL, e.g. https://cube.chrisproject.org/api/v1/pipelines/
#[braid(serde)]
pub struct PipelinesUrl;

#[braid(serde)]
pub struct PluginName;

#[braid(serde)]
pub struct PluginVersion;

#[braid(serde)]
pub struct ParameterName;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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
pub struct PipelineId(u16);

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

#[derive(Debug, Deserialize)]
pub struct PipelineUploadResponse {
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
    pub workflows: PipelineWorkflowsUrl
}
