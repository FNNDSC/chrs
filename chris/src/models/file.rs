use crate::models::Downloadable;
use crate::search::Search;
use crate::types::*;
use crate::Access;
use serde::Deserialize;
use time::OffsetDateTime;

/// The common data from any response object, and what comes back from the filebrowser API.
#[derive(Deserialize)]
pub struct BasicFileResponse {
    file_resource: FileResourceUrl,
    fname: FileResourceFname,
    fsize: u64,
}

/// A file created by a plugin instance.
#[derive(Deserialize)]
pub struct FeedFileResponse {
    pub url: ItemUrl,
    pub id: FeedFileId,
    #[serde(with = "time::serde::iso8601")]
    pub creation_date: OffsetDateTime,
    pub feed_id: FeedId,
    pub plugin_inst_id: PluginInstanceId,
    pub plugin_inst: ItemUrl,
    fname: FileResourceFname,
    fsize: u64,
    file_resource: FileResourceUrl,
}

/// A file uploaded to userfiles.
#[derive(Debug, Deserialize)]
pub struct FileUploadResponse {
    pub url: ItemUrl,
    pub id: u32,
    #[serde(with = "time::serde::iso8601")]
    pub creation_date: OffsetDateTime,
    fname: FileResourceFname,
    fsize: u64,
    file_resource: FileResourceUrl,
    pub owner: Username,
}

impl Downloadable for BasicFileResponse {
    fn file_resource_url(&self) -> &FileResourceUrl {
        &self.file_resource
    }

    fn fname(&self) -> &FileResourceFname {
        &self.fname
    }

    fn fsize(&self) -> u64 {
        self.fsize
    }
}

impl Downloadable for FileUploadResponse {
    fn file_resource_url(&self) -> &FileResourceUrl {
        &self.file_resource
    }

    fn fname(&self) -> &FileResourceFname {
        &self.fname
    }

    fn fsize(&self) -> u64 {
        self.fsize
    }
}

impl Downloadable for FeedFileResponse {
    fn file_resource_url(&self) -> &FileResourceUrl {
        &self.file_resource
    }

    fn fname(&self) -> &FileResourceFname {
        &self.fname
    }

    fn fsize(&self) -> u64 {
        self.fsize
    }
}

impl<A: Access> Search<FeedFileResponse, A> {
    /// Produce [BasicFileResponse] instead of [FeedFileResponse]
    pub fn basic(self) -> Search<BasicFileResponse, A> {
        self.downgrade()
    }
}

impl From<BasicFileResponse> for FileResourceFname {
    fn from(value: BasicFileResponse) -> Self {
        value.fname
    }
}
