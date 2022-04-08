use aliri_braid::braid;
use async_stream::stream;
use futures::Stream;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Query string parameters for paginated GET endpoints.
#[derive(Serialize)]
pub(crate) struct PaginationQuery {
    pub limit: u8,
    pub offset: u32,
}

/// An API URL which returns a paginated resource.
pub(crate) trait PaginatedUrl: AsRef<str> + Clone + DeserializeOwned {}

/// Create a [futures::Stream] that yields items from a paginated URL.
///
/// Limitation: Cannot produce [crate::client::CUBEError]
pub(crate) fn paginate<'a, U: 'a + PaginatedUrl, R: 'a + DeserializeOwned>(
    client: &'a reqwest::Client,
    url: &'a U,
) -> impl Stream<Item = Result<R, reqwest::Error>> + 'a {
    stream! {
        let mut next_url = Some(url.clone());
        while let Some(u) = next_url {
            let res = client.get(u.as_ref()).send().await?;
            let page: Paginated<U, R> = res.json().await?;

            for item in page.results {
                yield Ok(item);
            }
            next_url = page.next;
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct Paginated<U, T> {
    pub count: u32,
    pub next: Option<U>,
    pub previous: Option<U>,
    pub results: Vec<T>,
}

/// A URL which produces a collection of files.
///
/// # Examples
///
/// - https://cube.chrisproject.org/api/v1/files/
/// - https://cube.chrisproject.org/api/v1/files/search/
/// - https://cube.chrisproject.org/api/v1/uploadedfiles/search/?fname=txt
/// - https://cube.chrisproject.org/api/v1/20/files/
/// - https://cube.chrisproject.org/api/v1/plugins/instances/40/files/
#[braid(serde)]
pub struct AnyFilesUrl;
impl PaginatedUrl for AnyFilesUrl {}

/// Download URL for a file resource.
///
/// # Examples
///
/// - https://cube.chrisproject.org/api/v1/files/84360/aparc.a2009s+aseg.mgz
#[braid(serde)]
pub struct FileResourceUrl;

/// File fname.
#[braid(serde)]
pub struct FileResourceFname;

#[derive(Deserialize)]
pub struct DownloadableFile {
    pub file_resource: FileResourceUrl,
    pub fname: FileResourceFname,
}
