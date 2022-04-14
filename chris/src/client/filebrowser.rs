//! Beware: code quality here is much lower than the rest of the
//! code base because I'm running out of steam.

use crate::api::{DownloadableFile, FileBrowserUrl};
use crate::errors::{check, CUBEError};
use crate::pagination::{paginate, PaginatedUrl};
use aliri_braid::braid;
use futures::Stream;
use serde::Deserialize;
use serde::Serialize;

/// A client for the _ChRIS_ filebrowser API.
pub struct FileBrowser {
    client: reqwest::Client,
    search: FileBrowserSearchUrl,
}

#[braid(serde)]
pub(crate) struct FileBrowserSearchUrl;

/// A path which can be browsed by the file browser API.
#[braid(serde)]
pub struct FileBrowserPath;

impl From<FileBrowserUrl> for FileBrowserSearchUrl {
    fn from(url: FileBrowserUrl) -> Self {
        FileBrowserSearchUrl::new(format!("{}search/", url))
    }
}

impl FileBrowser {
    pub(crate) fn new(client: reqwest::Client, url: FileBrowserUrl) -> Self {
        FileBrowser {
            client,
            search: url.into(),
        }
    }

    pub async fn browse(
        &self,
        path: &FileBrowserPath,
    ) -> Result<Option<FileBrowserView>, CUBEError> {
        let res = self
            .client
            .get(self.search.as_str())
            .query(&FileBrowserQuery { path })
            .send()
            .await?;
        let mut data: FileBrowserSearch = check(res).await?.json().await?;
        if data.results.is_empty() {
            return Ok(None);
        }
        let dir = data.results.swap_remove(0);
        Ok(Some(FileBrowserView::new(dir, self.client.clone())))
    }
}

#[derive(Deserialize)]
struct FileBrowserSearch {
    // count: u8,
    // next: Option<String>,
    // previous: Option<String>,
    results: Vec<FileBrowserDir>,
}

#[derive(Deserialize)]
struct FileBrowserDir {
    path: FileBrowserPath,
    subfolders: String,
    // url: String,
    files: FileBrowserFilesUrl,
}

#[braid(serde)]
struct FileBrowserFilesUrl;

impl PaginatedUrl for FileBrowserFilesUrl {}

pub struct FileBrowserView {
    client: reqwest::Client,
    path: FileBrowserPath,
    subfolders: String,
    // url: String,
    files: FileBrowserFilesUrl,
}

impl FileBrowserView {
    fn new(dir: FileBrowserDir, client: reqwest::Client) -> Self {
        FileBrowserView {
            client,
            path: dir.path,
            subfolders: dir.subfolders,
            // url: dir.url,
            files: dir.files,
        }
    }

    /// Get the current path.
    pub fn path(&self) -> &FileBrowserPath {
        &self.path
    }

    /// Produce the subpaths from this level's subfolders.
    pub fn subpaths(&self) -> impl Iterator<Item = FileBrowserPath> + '_ {
        self.subfolders()
            .into_iter()
            .map(|subfolder| format!("{}/{}", self.path, subfolder))
            .map(FileBrowserPath::new)
    }

    /// Iterate over subfolders.
    ///
    /// WARNING: subfolders are comma-separated values, so paths containing
    /// commas will cause glitches!
    pub fn subfolders(&self) -> impl Iterator<Item = &str> {
        self.subfolders.split_terminator(',')
    }

    /// Iterate over files.
    pub fn iter_files(&self) -> impl Stream<Item = Result<DownloadableFile, reqwest::Error>> + '_ {
        paginate(&self.client, &self.files)
    }
}

#[derive(Serialize)]
struct FileBrowserQuery<'a> {
    path: &'a FileBrowserPath,
}
