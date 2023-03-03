use crate::errors::{check, CUBEError, FileIOError};
use crate::models::data::*;
use crate::models::ConnectedModel;
use fs_err::tokio::{File, OpenOptions};
use futures::{Stream, TryStreamExt};
use serde::de::DeserializeOwned;
use std::path::Path;
use tokio_util::io::StreamReader;

/// A CUBE resource which has `file_resource`, `fname`, and `fsize`.
pub trait Downloadable {
    fn file_resource(&self) -> &FileResourceUrl;
    fn fname(&self) -> &FileResourceFname;
    fn fsize(&self) -> u64;
}

impl<D: Downloadable + DeserializeOwned> ConnectedModel<D> {
    /// Stream the bytes data of a file from _ChRIS_.
    /// Returns the bytestream and content-length.
    pub async fn stream(
        &self,
    ) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, CUBEError> {
        let res = self
            .client
            .get(self.data.file_resource().as_str())
            .send()
            .await?;
        let stream = check(res).await?.bytes_stream();
        Ok(stream)
    }

    /// Download a file from _ChRIS_ to a local path.
    pub async fn download(&self, dst: &Path, clobber: bool) -> Result<(), FileIOError> {
        let mut file = if clobber {
            File::create(dst).await
        } else {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dst)
                .await
        }
        .map_err(FileIOError::IO)?;
        let stream = self
            .stream()
            .await?
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e));
        let mut reader = StreamReader::new(stream);
        tokio::io::copy(&mut reader, &mut file).await?;
        Ok(())
    }
}

// ============================================================
//                     GETTER METHODS
// ============================================================

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
