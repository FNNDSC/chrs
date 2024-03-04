//! Errors for this crate.
//! About anyhow: see https://github.com/TrueLayer/reqwest-middleware/issues/119

use reqwest::StatusCode;

#[derive(thiserror::Error, Debug)]
pub enum InvalidCubeUrl {
    #[error("Given URL does not end with \"/api/v1/\": {0}")]
    EndpointVersion(String),

    #[error("Given URL does not start with \"http://\" or \"https://\": {0}")]
    Protocol(String),
}

aliri_braid::from_infallible!(InvalidCubeUrl);

/// Errors representing failed interactions with CUBE.
#[derive(thiserror::Error, Debug)]
pub enum CubeError {
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

    /// Error from reqwest middleware function.
    #[error(transparent)]
    Middleware(anyhow::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum GetError {
    #[error(transparent)]
    CUBEError(#[from] CubeError),

    /// Error when trying to get an object but it is not found.
    #[error("\"{0}\" not found")]
    NotFound(String),
}

pub(crate) async fn check(res: reqwest::Response) -> Result<reqwest::Response, CubeError> {
    match res.error_for_status_ref() {
        Ok(_) => Ok(res),
        Err(source) => {
            let status = res.status();
            let reason = status.canonical_reason().unwrap_or("unknown reason");
            let text = res.text().await.map_err(CubeError::Raw)?;
            Err(CubeError::Error {
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
    Cube(CubeError),
    #[error(transparent)]
    IO(std::io::Error),
}

impl From<reqwest::Error> for FileIOError {
    fn from(e: reqwest::Error) -> Self {
        FileIOError::Cube(CubeError::Raw(e))
    }
}

impl From<reqwest_middleware::Error> for FileIOError {
    fn from(e: reqwest_middleware::Error) -> Self {
        FileIOError::Cube(e.into())
    }
}

impl From<CubeError> for FileIOError {
    fn from(e: CubeError) -> Self {
        FileIOError::Cube(e)
    }
}

impl From<std::io::Error> for FileIOError {
    fn from(e: std::io::Error) -> Self {
        FileIOError::IO(e)
    }
}

impl From<reqwest_middleware::Error> for CubeError {
    fn from(error: reqwest_middleware::Error) -> Self {
        match error {
            reqwest_middleware::Error::Middleware(e) => CubeError::Middleware(e),
            reqwest_middleware::Error::Reqwest(e) => CubeError::Raw(e),
        }
    }
}
