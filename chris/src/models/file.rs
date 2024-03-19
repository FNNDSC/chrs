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

/// A PACSFile.
#[derive(Debug, Deserialize)]
pub struct PacsFileResponse {
    pub url: ItemUrl,
    pub id: PacsFileId,
    #[serde(with = "time::serde::iso8601")]
    pub creation_date: OffsetDateTime,
    pub fname: FileResourceFname,
    pub fsize: u64,

    pub file_resource: FileResourceUrl,
    pub pacs_identifier: String,

    #[serde(rename = "PatientID")]
    pub patient_id: String,
    #[serde(rename = "StudyDate")]
    pub study_date: String,
    #[serde(rename = "StudyInstanceUID")]
    pub study_instance_uid: String,
    #[serde(rename = "SeriesInstanceUID")]
    pub series_instance_uid: String,
    #[serde(rename = "PatientName")]
    pub patient_name: Option<String>,
    #[serde(rename = "PatientBirthDate")]
    pub patient_birth_date: Option<String>,
    #[serde(rename = "PatientAge")]
    pub patient_age: Option<u32>,
    #[serde(rename = "PatientSex")]
    pub patient_sex: Option<String>,
    #[serde(rename = "AccessionNumber")]
    pub accession_number: Option<String>,
    #[serde(rename = "Modality")]
    pub modality: Option<String>,
    #[serde(rename = "ProtocolName")]
    pub protocol_name: Option<String>,
    #[serde(rename = "StudyDescription")]
    pub study_description: Option<String>,
    #[serde(rename = "SeriesDescription")]
    pub series_description: Option<String>,
}

// TODO can I write a derive macro for Downloadable?

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

impl Downloadable for PacsFileResponse {
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
