use crate::api::{PluginName, PluginVersion};
use reqwest::StatusCode;

/// Errors representing failed interactions with CUBE.
#[derive(thiserror::Error, Debug)]
pub enum CUBEError {
    /// Error response with an explanation from CUBE.
    #[error("({status:?} {reason:?}): {text}")]
    Error {
        status: StatusCode,
        reason: &'static str,
        text: String,
        source: reqwest::Error,
    },

    /// Error response without explanation from CUBE (badness 100000).
    #[error(transparent)]
    Raw(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum DircopyError {
    #[error(transparent)]
    CUBEError(#[from] CUBEError),

    #[error("\"{0}\" version {1} not found")]
    DircopyNotFound(&'static PluginName, &'static PluginVersion),
}

pub(crate) async fn check(res: reqwest::Response) -> Result<reqwest::Response, CUBEError> {
    match res.error_for_status_ref() {
        Ok(_) => Ok(res),
        Err(source) => {
            let status = res.status();
            let reason = status.canonical_reason().unwrap_or("unknown reason");
            let text = res.text().await.map_err(CUBEError::Raw)?;
            Err(CUBEError::Error {
                status,
                reason,
                text,
                source,
            })
        }
    }
}

/// An error which might occur while uploading or downloading files.
#[derive(thiserror::Error, Debug)]
pub enum FileIOError {
    #[error("\"{0}\" is an invalid file path")]
    PathError(String),
    #[error(transparent)]
    Cube(CUBEError),
    #[error(transparent)]
    IO(std::io::Error),
}

impl From<reqwest::Error> for FileIOError {
    fn from(e: reqwest::Error) -> Self {
        FileIOError::Cube(CUBEError::Raw(e))
    }
}

impl From<CUBEError> for FileIOError {
    fn from(e: CUBEError) -> Self {
        FileIOError::Cube(e)
    }
}

impl From<std::io::Error> for FileIOError {
    fn from(e: std::io::Error) -> Self {
        FileIOError::IO(e)
    }
}
