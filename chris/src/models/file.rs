use crate::models::Downloadable;
use crate::types::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CubeFile {
    file_resource: FileResourceUrl,
    fname: FileResourceFname,
    fsize: u64,
}

impl From<CubeFile> for FileResourceFname {
    fn from(value: CubeFile) -> Self {
        value.fname
    }
}

impl Downloadable for CubeFile {
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
    pub creation_date: DateString,
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
