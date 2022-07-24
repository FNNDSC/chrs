use crate::executor::do_with_progress;
use anyhow::{bail, Context};
use async_stream::stream;
use chris::api::{AnyFilesUrl, Downloadable, FileResourceFname};
use chris::common_types::CUBEApiUrl;
use chris::ChrisClient;
use std::path::{Path, PathBuf};
use tokio::fs;
use url::Url;

/// `chrs download` command.
pub(crate) async fn download(
    client: &ChrisClient,
    src: &str,
    dst: Option<&Path>,
    shorten: u8,
) -> anyhow::Result<()> {
    let dst = choose_dst(client.url(), src, dst);

    let url = parse_src(src, client.url());
    let count = client.get_count(url.as_str()).await.with_context(|| {
        format!(
            "Could not get count of files from {} -- is it a files URL?",
            url
        )
    })?;
    if count == 0 {
        bail!("No files found under {} (resolved as {})", src, url);
    }

    if count == 1 {
        download_single_file(client, &url, src, &dst, shorten).await
    } else {
        download_directory(client, &url, src, &dst, shorten, count).await
    }
}

/// Download a single file from a ChRIS files URL.
/// The given `url` is assumed to be a collection API endpoint with exactly one item.
async fn download_single_file<'a>(
    chris: &'a ChrisClient,
    url: &'a AnyFilesUrl,
    src: &'a str,
    dst: &'a Path,
    shorten: u8
) -> anyhow::Result<()> {
    todo!()
}

/// Download all files from a ChRIS files URL.
/// The given `url` is assumed to be of a collection API endpoint with more than one item.
async fn download_directory<'a>(
    chris: &'a ChrisClient,
    url: &'a AnyFilesUrl,
    src: &'a str,
    dst: &'a Path,
    shorten: u8,
    count: u32,
) -> anyhow::Result<()> {
    if dst.exists() && !dst.is_dir() {
        bail!("Not a directory: {:?}", dst);
    }
    let parent_len = dir_length_of(chris.url(), src);

    let stream = stream! {
        for await page in chris.iter_files(url) {
            yield page
                .map(|d| download_helper(chris, d, dst, parent_len, shorten))
                .map_err(DownloadError::Pagination)
        }
    };

    do_with_progress(stream, count as u64, false).await?;
    Ok(())
}

/// Decide where to save output files to.
fn choose_dst(url: &CUBEApiUrl, src: &str, dst: Option<&Path>) -> PathBuf {
    if let Some(given_dst) = dst {
        return given_dst.to_path_buf();
    }
    if src.starts_with(url.as_str()) {
        return PathBuf::from(".");
    }
    let (_parent, basename) = split_path(src);
    PathBuf::from(basename)
}

/// Figure out whether the input is a URL or a path.
/// If it's a path, then construct a search URL from it.
///
/// Returns the URL and the length of the given fname, or 0
/// if not given an fname.
fn parse_src(src: &str, address: &CUBEApiUrl) -> AnyFilesUrl {
    if src.starts_with(address.as_str()) {
        return src.into();
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

/// Create a search API URL for the endpoint and fname.
fn to_search(address: &CUBEApiUrl, endpoint: &str, fname: &str) -> AnyFilesUrl {
    Url::parse_with_params(
        &*format!("{}{}/search/", address, endpoint),
        &[("fname", fname)],
    )
    .unwrap()
    .as_str()
    .into()
}

/// If src is a path, assume it's a directory and
/// return the length of its name with a trailing slash.
fn dir_length_of(address: &CUBEApiUrl, src: &str) -> usize {
    if src.starts_with(address.as_str()) {
        0
    } else if src.ends_with("/") {
        src.len()
    } else {
        src.len() + 1
    }
}

/// Return the parent directory and basename of a given path.
fn split_path(src: &str) -> (&str, &str) {
    let canon_fname = src.strip_suffix("/").unwrap_or(src);
    if let Some(t) = canon_fname.rsplit_once("/") {
        t
    } else {
        ("", canon_fname)
    }
}

/// Download a file to a destination, creating parent directories as needed.
async fn download_helper(
    client: &ChrisClient,
    downloadable: impl Downloadable,
    dst: &Path,
    parent_len: usize,
    shorten: u8,
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
        .map_err(|e| DownloadError::IO {
            fname: downloadable.fname().to_owned(),
            path: dst.as_path().to_path_buf(),
            source: e,
        })
}

/// Choose path on host where to save the file.
fn decide_target(fname: &FileResourceFname, dst: &Path, parent_len: usize, shorten: u8) -> PathBuf {
    let fname: &str = fname.as_str();
    let base = &fname[parent_len..fname.len()];
    let shortened = shorten_target(base, shorten);
    dst.join(shortened)
}

/// Shorten a fname by truncating the parent directories before and including "/data/"
fn shorten_target(path: &str, shorten: u8) -> &str {
    if shorten == 0 {
        return path;
    }
    if let Some((_, s)) = path.split_once("/data/") {
        return shorten_target(s, shorten - 1);
    }
    return path;
}

/// Errors which might occur when trying to download many files from
/// a collection URL.
#[derive(thiserror::Error, Debug)]
enum DownloadError {
    /// Error from paginating the given URL.
    #[error(transparent)]
    Pagination(#[from] reqwest::Error),

    /// Error from downloading from a `file_resource`.
    #[error("Cannot write \"{fname}\" to \"{path}\": {source}")]
    IO {
        fname: FileResourceFname,
        path: PathBuf,
        #[source]
        source: chris::errors::FileIOError,
    },

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
    #[case("chris/uploads", "chris", "uploads")]
    #[case("chris/uploads/my_study", "chris/uploads", "my_study")]
    #[case("chris/uploads/my_study/", "chris/uploads", "my_study")]
    #[case("chris", "", "chris")]
    #[case("", "", "")]
    fn test_split_path(#[case] src: &str, #[case] parent: &str, #[case] basename: &str) {
        assert_eq!(split_path(src), (parent, basename))
    }

    #[rstest]
    #[case(
        "https://example.com/api/v1/files/search/?fname_icontains=gluten",
        None,
        "."
    )]
    #[case(
        "https://example.com/api/v1/files/search/?fname_icontains=gluten",
        Some("my_output"),
        "my_output"
    )]
    #[case("iamme/uploads", Some("my_output"), "my_output")]
    #[case("iamme/uploads", None, "uploads")]
    #[case("iamme/uploads/", None, "uploads")]
    fn test_choose_dst(
        example_address: &CUBEApiUrl,
        #[case] src: &str,
        #[case] dst: Option<&str>,
        #[case] expected: &str,
    ) {
        let actual = choose_dst(example_address, src, dst.map(Path::new));
        assert_eq!(actual.as_path(), Path::new(expected))
    }

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
    fn test_parse_src_url(#[case] src: &str, #[case] expected: &str, example_address: &CUBEApiUrl) {
        assert_eq!(parse_src(src, example_address), AnyFilesUrl::from(expected));
    }

    #[rstest]
    #[case(
        "https://example.com/api/v1/files/search/?fname=cereal%2Ffeed_1%2Fpl-dircopy_1",
        0
    )]
    #[case("chris/feed_14/data/pl-brainmgz_14/data", 39)]
    #[case("chris/feed_14/data/pl-brainmgz_14/data/", 39)]
    fn test_dir_length_of(
        example_address: &CUBEApiUrl,
        #[case] src: &str,
        #[case] expected: usize,
    ) {
        let actual = dir_length_of(example_address, src);
        assert_eq!(actual, expected);
    }

    #[fixture]
    #[once]
    fn example_address() -> CUBEApiUrl {
        CUBEApiUrl::try_from("https://example.com/api/v1/").unwrap()
    }

    #[rstest]
    #[case("chris/uploads/brain.nii", ".", 0, 0, "./chris/uploads/brain.nii")]
    #[case(
        "chris/uploads/brain.nii",
        "output",
        0,
        0,
        "output/chris/uploads/brain.nii"
    )]
    #[case("chris/uploads/brain.nii", "output", 14, 0, "output/brain.nii")]
    #[case(
        "chris/feed_1/pl-dircopy/data/brain.nii",
        "output",
        0,
        1,
        "output/brain.nii"
    )]
    fn test_decide_target(
        #[case] fname: &str,
        #[case] dst: &str,
        #[case] parent_len: usize,
        #[case] shorten: u8,
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

    #[rstest]
    #[case("chris/uploads/brain.nii", 0, "chris/uploads/brain.nii")]
    #[case("chris/uploads/brain.nii", 1, "chris/uploads/brain.nii")]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/something.txt",
        0,
        "jennings/feed_82/pl-dircopy_532/data/something.txt"
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/something.txt",
        1,
        "something.txt"
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/something.txt",
        5,
        "something.txt"
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt",
        0,
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt"
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt",
        1,
        "rudolphs_change_goes_here/data/something.txt"
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt",
        2,
        "something.txt"
    )]
    fn test_shorten_target(#[case] fname: &str, #[case] shorten: u8, #[case] expected: &str) {
        assert_eq!(shorten_target(fname, shorten), expected)
    }
}
