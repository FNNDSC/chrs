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
    pub files: AllFilesUrl,
    pub uploadedfiles: UploadedFilesUrl,
    pub user: UserUrl,
    pub pipelines: PipelinesUrl,
}

/// CUBE files resource URL, e.g. `https://cube.chrisproject.org/api/v1/files/`
#[braid(serde)]
pub struct AllFilesUrl;

/// CUBE uploaded files resource URL, e.g. `https://cube.chrisproject.org/api/v1/uploadedfiles/`
#[braid(serde)]
pub struct UploadedFilesUrl;

/// CUBE User ID
#[derive(Shrinkwrap, Deserialize)]
pub struct UserId(u16);

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
}

#[derive(Deserialize)]
pub struct DownloadableFile {
    file_resource: FileResourceUrl,
    fname: FileResourceFname,
}

impl Downloadable for DownloadableFile {
    fn file_resource(&self) -> &FileResourceUrl {
        &self.file_resource
    }

    fn fname(&self) -> &FileResourceFname {
        &self.fname
    }
}

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
    pub workflows: PipelineWorkflowsUrl,
}

#[derive(Debug, Deserialize)]
pub struct FileUploadResponse {
    pub url: String,
    pub id: u32,
    pub creation_date: String,
    fname: FileResourceFname,
    pub fsize: u32,
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
}
