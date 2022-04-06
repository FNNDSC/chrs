use aliri_braid::braid;
use serde::{Deserialize, Serialize};

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
