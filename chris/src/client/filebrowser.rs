//! Beware: code quality here is much lower than the rest of the
//! code base because I'm running out of steam.

use crate::errors::{check, CUBEError};
use crate::models::{DownloadableFile, FileBrowserUrl, FileResourceFname};
use crate::pagination::{paginate, PaginatedUrl};
use aliri_braid::braid;
use futures::Stream;
use serde::Deserialize;
use serde::Serialize;
use serde_with::json::JsonString;
use serde_with::serde_as;

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

impl From<FileBrowserPath> for FileResourceFname {
    fn from(p: FileBrowserPath) -> Self {
        FileResourceFname::new(p.to_string())
    }
}

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

#[serde_as]
#[derive(Deserialize)]
struct FileBrowserDir {
    path: FileBrowserPath,
    #[serde_as(as = "JsonString")]
    subfolders: Vec<String>,
    // url: String,
    files: Option<FileBrowserFilesUrl>,
}

#[braid(serde)]
struct FileBrowserFilesUrl;

impl PaginatedUrl for FileBrowserFilesUrl {}

pub struct FileBrowserView {
    client: reqwest::Client,
    path: FileBrowserPath,
    subfolders: Vec<String>,
    // url: String,
    /// API Url for files immediately under this path.
    /// Is `None` if path is `""` (root).
    files: Option<FileBrowserFilesUrl>,
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

    /// Iterate over subfolders.
    pub fn subfolders(&self) -> &Vec<String> {
        &self.subfolders
    }

    /// Produce the subpaths from this level's subfolders.
    pub fn subpaths(&self) -> impl Iterator<Item = FileBrowserPath> + '_ {
        self.subfolders()
            .into_iter()
            .map(|subfolder| format!("{}/{}", self.path, subfolder))
            .map(FileBrowserPath::new)
    }

    /// Iterate over files.
    pub fn iter_files(&self) -> impl Stream<Item = Result<DownloadableFile, reqwest::Error>> + '_ {
        paginate(&self.client, self.files.clone())
    }
}

#[derive(Serialize)]
struct FileBrowserQuery<'a> {
    path: &'a FileBrowserPath,
}
