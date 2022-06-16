use crate::executor::do_with_progress;
use anyhow::{bail, Context};
use async_stream::stream;
use chris::api::{AnyFilesUrl, Downloadable, FileResourceFname};
use chris::common_types::CUBEApiUrl;
use chris::ChrisClient;
use futures::Stream;
use std::future::Future;
use std::path::{Path, PathBuf};
use tokio::fs;
use url::Url;

pub(crate) async fn download(
    client: &ChrisClient,
    src: &str,
    dst: &Path,
    shorten: bool,
) -> anyhow::Result<()> {
    if dst.exists() && !dst.is_dir() {
        bail!("Not a directory: {:?}", dst);
    }

    let (url, parent_len) = parse_src(src, client.url());
    let count = client.get_count(url.as_str()).await.with_context(|| {
        format!(
            "Could not get count of files from {} -- is it a files URL?",
            url
        )
    })?;
    if count == 0 {
        bail!("No files found under {} (resolved as {})", src, url);
    }

    let stream = stream2download(client, &url, dst, parent_len, shorten);
    do_with_progress(stream, count as u64, false).await?;
    anyhow::Ok(())
}

/// Figure out whether the input is a URL or a path.
/// If it's a path, then construct a search URL from it.
///
/// Returns the URL and the length of the given fname, or 0
/// if not given an fname.
fn parse_src(src: &str, address: &CUBEApiUrl) -> (AnyFilesUrl, usize) {
    if src.starts_with(address.as_str()) {
        return (AnyFilesUrl::from(src), 0);
    }
    if src.starts_with("SERVICES") {
        if src.starts_with("SERVICES/PACS") {
            return to_search(address, "pacsfiles", src);
        }
        return to_search(address, "servicefiles", src);
    }
    if let Some((_username, subdir)) = src.split_once('/') {
        if subdir.starts_with("uploads") {
            return to_search(address, "uploadedfiles", src);
        }
    }
    to_search(address, "files", src)
}

fn to_search(address: &CUBEApiUrl, endpoint: &str, fname: &str) -> (AnyFilesUrl, usize) {
    let url = Url::parse_with_params(
        &*format!("{}{}/search/", address, endpoint),
        &[("fname", fname)],
    )
    .unwrap();

    // Return length of the parent dir.
    // Later on, the parent dir is truncated by that len, so that
    // if a user wants to download all files under the parent dir
    // "chris/uploads" or "chris/uploads/", the destination paths
    // are file resource fnames without the leading
    // "chris/uploads/" prefix.
    let slash_len = if fname.ends_with('/') { 0 } else { 1 };
    (AnyFilesUrl::from(url.as_str()), fname.len() + slash_len)
}

fn stream2download<'a>(
    client: &'a ChrisClient,
    url: &'a AnyFilesUrl,
    dst: &'a Path,
    parent_len: usize,
    shorten: bool,
) -> impl Stream<Item = Result<impl Future<Output = Result<(), DownloadError>> + 'a, DownloadError>> + 'a
{
    stream! {
        for await page in client.iter_files(url) {
            yield match page {
                Err(e) => Err(DownloadError::Pagination(e)),
                Ok(downloadable) => {
                    Ok(download_helper(client, downloadable, dst, parent_len, shorten))
                }
            };
        }
    }
}

async fn download_helper(
    client: &ChrisClient,
    downloadable: impl Downloadable,
    dst: &Path,
    parent_len: usize,
    shorten: bool,
) -> Result<(), DownloadError> {
    let dst = decide_target(downloadable.fname(), dst, parent_len, shorten);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| DownloadError::ParentDirectory {
                path: parent.to_path_buf(),
                source: e,
            })?;
    }
    client
        .download_file(&downloadable, dst.as_path(), false)
        .await
        .map_err(|e| e.into())
}

fn decide_target(
    fname: &FileResourceFname,
    dst: &Path,
    parent_len: usize,
    shorten: bool,
) -> PathBuf {
    let fname: &str = fname.as_str();
    let mut shortened = &fname[parent_len..fname.len()];
    if shorten {
        if let Some((_, s)) = shortened.split_once("/data/") {
            shortened = s;
        }
    }
    dst.join(shortened)
}

/// Errors which might occur when trying to download many files from
/// a collection URL.
#[derive(thiserror::Error, Debug)]
enum DownloadError {
    /// Error from paginating the given URL.
    #[error(transparent)]
    Pagination(#[from] reqwest::Error),

    /// Error from downloading from a `file_resource`.
    #[error(transparent)]
    Download(#[from] chris::errors::FileIOError),

    #[error("Unable to create directory: {path:?}")]
    ParentDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case(
        "https://example.com/api/v1/uploadedfiles/search/?fname_icontains=gluten",
        "https://example.com/api/v1/uploadedfiles/search/?fname_icontains=gluten"
    )]
    #[case(
        "SERVICES/PACS/orthanc",
        "https://example.com/api/v1/pacsfiles/search/?fname=SERVICES%2FPACS%2Forthanc"
    )]
    #[case(
        "waffle/uploads/powdered_sugar",
        "https://example.com/api/v1/uploadedfiles/search/?fname=waffle%2Fuploads%2Fpowdered_sugar"
    )]
    #[case(
        "cereal/feed_1/pl-dircopy_1",
        "https://example.com/api/v1/files/search/?fname=cereal%2Ffeed_1%2Fpl-dircopy_1"
    )]
    fn test_parse_src(#[case] src: &str, #[case] expected: &str, example_address: &CUBEApiUrl) {
        assert_eq!(
            parse_src(src, example_address).0,
            AnyFilesUrl::from(expected)
        );
    }

    #[rstest]
    #[case("cereal/feed_1/pl-dircopy_1", 27)]
    #[case("cereal/feed_1/pl-dircopy_1/", 27)]
    fn test_parse_src_len(
        #[case] src: &str,
        #[case] expected: usize,
        example_address: &CUBEApiUrl,
    ) {
        assert_eq!(parse_src(src, example_address).1, expected);
    }

    #[fixture]
    #[once]
    fn example_address() -> CUBEApiUrl {
        CUBEApiUrl::try_from("https://example.com/api/v1/").unwrap()
    }

    #[rstest]
    #[case("chris/uploads/brain.nii", ".", 0, false, "./chris/uploads/brain.nii")]
    #[case(
        "chris/uploads/brain.nii",
        "output",
        0,
        false,
        "output/chris/uploads/brain.nii"
    )]
    #[case("chris/uploads/brain.nii", "output", 14, false, "output/brain.nii")]
    #[case(
        "chris/feed_1/pl-dircopy/data/brain.nii",
        "output",
        0,
        true,
        "output/brain.nii"
    )]
    fn test_decide_target(
        #[case] fname: &str,
        #[case] dst: &str,
        #[case] parent_len: usize,
        #[case] shorten: bool,
        #[case] expected: &str,
    ) {
        assert_eq!(
            decide_target(
                &FileResourceFname::from(fname),
                Path::new(dst),
                parent_len,
                shorten
            ),
            PathBuf::from(expected)
        )
    }
}
