use crate::models::data::*;
use crate::models::ConnectedModel;
use serde::de::DeserializeOwned;

/// A CUBE resource which has `file_resource`, `fname`, and `fsize`.
pub trait Downloadable {
    fn file_resource(&self) -> &FileResourceUrl;
    fn fname(&self) -> &FileResourceFname;
    fn fsize(&self) -> u64;
}

impl<D: Downloadable + DeserializeOwned> ConnectedModel<D> {}

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
