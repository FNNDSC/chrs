use crate::models::Downloadable;
use crate::types::*;
use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Deserialize)]
pub struct BasicFileResponse {
    file_resource: FileResourceUrl,
    fname: FileResourceFname,
    fsize: u64,
}

impl From<BasicFileResponse> for FileResourceFname {
    fn from(value: BasicFileResponse) -> Self {
        value.fname
    }
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
