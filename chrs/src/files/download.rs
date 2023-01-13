use crate::executor::do_with_progress;
use crate::files::human_paths::MaybeNamer;
use crate::io_helper::progress_bar_bytes;
use anyhow::{bail, Context};
use async_stream::try_stream;
use chris::common_types::CUBEApiUrl;
use chris::models::{AnyFilesUrl, Downloadable, DownloadableFile, FileResourceFname};
use chris::ChrisClient;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::File;
use tokio_util::io::StreamReader;
use url::Url;

/// `chrs download` command.
pub(crate) async fn download(
    client: &ChrisClient,
    src: &str,
    dst: Option<&Path>,
    shorten: u8,
    rename: bool,
    flatten: bool
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
        download_single_file(client, &url, &dst).await
    } else {
        download_directory(client, &url, src, &dst, shorten, rename, count).await
    }
}

/// Download a single file from a ChRIS files URL.
/// The given `url` is assumed to be a collection API endpoint with exactly one item.
async fn download_single_file<'a>(
    chris: &'a ChrisClient,
    url: &'a AnyFilesUrl,
    dst: &'a Path,
) -> anyhow::Result<()> {
    let file = File::create(dst)
        .await
        .with_context(|| format!("Cannot open {:?} for writing", dst))?;
    let downloadable = peek_file(chris, url).await?;
    download_to_file_with_progress(chris, &downloadable, file).await?;
    Ok(())
}

/// Download a file from _ChRIS_ to an open file using streaming with a progress bar.
async fn download_to_file_with_progress(
    chris: &ChrisClient,
    downloadable: &DownloadableFile,
    mut file: File,
) -> anyhow::Result<()> {
    let bar = progress_bar_bytes(downloadable.fsize());
    let stream = chris
        .stream_file(downloadable)
        .await?
        .map_ok(|bytes| {
            bar.inc(bytes.len() as u64);
            bytes
        })
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e));
    let mut reader = StreamReader::new(stream);
    tokio::io::copy(&mut reader, &mut file).await?;
    bar.finish();
    Ok(())
}

async fn peek_file(chris: &ChrisClient, url: &AnyFilesUrl) -> anyhow::Result<DownloadableFile> {
    let files = chris.iter_files(url);
    pin_mut!(files);
    let downloadable = files.next().await.ok_or_else(|| {
        anyhow::Error::msg(format!(
            "BACKEND BUG: count>0 but results are empty: {}\n",
            url
        ))
    })??;
    Ok(downloadable)
}

/// Download all files from a ChRIS files URL.
/// The given `url` is assumed to be of a collection API endpoint with more than one item.
async fn download_directory<'a>(
    chris: &'a ChrisClient,
    url: &'a AnyFilesUrl,
    src: &'a str,
    dst: &'a Path,
    shorten: u8,
    rename: bool,
    count: u32,
) -> anyhow::Result<()> {
    if dst.exists() && !dst.is_dir() {
        bail!("Not a directory: {:?}", dst);
    }

    let stream = get_downloads(chris, url, src, dst, shorten, rename)
        .map_ok(|(file, target)| download_helper(chris, file, target))
        .map_err(DownloadError::Pagination);

    do_with_progress(stream, count as u64, false).await?;
    Ok(())
}

/// Produce a stream of [DownloadableFile] resources from `url` and
/// a path on the filesystem where the file should be downloaded to.
fn get_downloads<'a>(
    chris: &'a ChrisClient,
    url: &'a AnyFilesUrl,
    src: &'a str,
    dst: &'a Path,
    shorten: u8,
    rename: bool,
) -> impl Stream<Item = Result<(DownloadableFile, PathBuf), reqwest::Error>> + 'a {
    let mut namer = MaybeNamer::new(chris, rename);
    let folder = FolderOutputOptions::new(chris.url(), src, dst, shorten);

    try_stream! {
        for await result in chris.iter_files(url) {
            let file = result?;
            let target = folder.where_to_download(file.fname(), &mut namer).await;
            yield (file, target)
        }
    }
}

/// While trying to figure out where to download a [FileResourceFname],
/// substrings might be removed from the front, making it no longer a
/// valid [FileResourceName]. Invalid fname are represented by
/// [ProcessedFname::Changed].
#[derive(Debug, Eq, PartialEq)]
enum ProcessedFname {
    Changed(String),
    Unchanged(FileResourceFname),
}

impl ProcessedFname {
    fn new(fname: FileResourceFname) -> Self {
        ProcessedFname::Unchanged(fname)
    }

    fn as_str(&self) -> &str {
        match self {
            ProcessedFname::Changed(f) => f,
            ProcessedFname::Unchanged(f) => f.as_str(),
        }
    }

    /// Shorten a fname by truncating the parent directories before and including "/data/"
    fn shorten(self, times: u8) -> Self {
        if times == 0 {
            return self;
        }
        if let Some((_, half)) = self.as_str().split_once("/data/") {
            ProcessedFname::Changed(half.to_string()).shorten(times - 1)
        } else {
            self
        }
    }

    /// Take the string to the right of an index position.
    fn substr_from(self, index: usize) -> Self {
        if index == 0 {
            self
        } else {
            let s = self.as_str();
            let rel = &s[index..s.len()];
            ProcessedFname::Changed(rel.to_string())
        }
    }

    async fn rename_using(self, namer: &mut MaybeNamer) -> String {
        match self {
            ProcessedFname::Changed(f) => rename_shortened(namer, &f).await,
            ProcessedFname::Unchanged(f) => namer.rename(&f).await,
        }
    }
}

struct FolderOutputOptions<'a> {
    parent_len: usize,
    // namer: Cell<MaybeNamer>,
    shorten: u8,
    output_dir: &'a Path,
}

impl<'a> FolderOutputOptions<'a> {
    fn new(address: &CUBEApiUrl, src: &str, output_dir: &'a Path, shorten: u8) -> Self {
        Self {
            // alternatively, we could use a mutable field for slightly cleaner code
            // namer: Cell::new(MaybeNamer::new(chris, rename)),
            parent_len: dir_length_of(address, src),
            shorten,
            output_dir,
        }
    }

    /// Decide on the path on the filesystem where to download a given fname.
    async fn where_to_download(
        &self,
        fname: &FileResourceFname,
        namer: &mut MaybeNamer,
    ) -> PathBuf {
        let base = self.relative_from_src_dir(fname);
        let shortened = base.shorten(self.shorten);
        let renamed = shortened.rename_using(namer).await;
        self.output_dir.join(renamed)
    }

    fn relative_from_src_dir(&'a self, fname: &'a FileResourceFname) -> ProcessedFname {
        let fname = ProcessedFname::new(fname.clone());
        fname.substr_from(self.parent_len)
    }
}

/// Rename the plugin instance folder names of `fname`.
///
/// Parent folders of the left-most occurrence of "/data/" are assumed to be
/// plugin instance folders. In come cases, such as when you run pl-dircopy
/// on an existing plugin instance's "/data/" folder, that plugin instance's
/// folders may appear after the first occurrence of "/data/". Those folders
/// _will not_ be renamed, since they are actually within the plugin instance's
/// output space, not part of its files' prefix!
async fn rename_shortened(namer: &mut MaybeNamer, fname: &str) -> String {
    if !fname.contains("/data/") {
        return fname.to_string();
    }
    namer.rename_plugin_instances(fname.split('/')).await
}

/// Download a file to a destination, creating parent directories as needed.
async fn download_helper(
    client: &ChrisClient,
    downloadable: impl Downloadable,
    dst: PathBuf,
) -> Result<(), DownloadError> {
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
            fname: downloadable.fname().clone(),
            path: dst,
            source: e,
        })
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
        &format!("{}{}/search/", address, endpoint),
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
    } else if src.ends_with('/') {
        src.len()
    } else {
        src.len() + 1
    }
}

/// Return the parent directory and basename of a given path.
fn split_path(src: &str) -> (&str, &str) {
    let canon_fname = src.strip_suffix('/').unwrap_or(src);
    if let Some(t) = canon_fname.rsplit_once('/') {
        t
    } else {
        ("", canon_fname)
    }
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
    #[case("chris/uploads/brain.nii", 0, ProcessedFname::Unchanged("chris/uploads/brain.nii".into()))]
    #[case("chris/uploads/brain.nii", 1, ProcessedFname::Unchanged("chris/uploads/brain.nii".into()))]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/something.txt",
        0,
    ProcessedFname::Unchanged("jennings/feed_82/pl-dircopy_532/data/something.txt".into())
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/something.txt",
        1,
        ProcessedFname::Changed("something.txt".to_string())
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/something.txt",
        5,
        ProcessedFname::Changed("something.txt".to_string())
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt",
        0,
        ProcessedFname::Unchanged(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt".into()
        )
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt",
        1,
        ProcessedFname::Changed("rudolphs_change_goes_here/data/something.txt".to_string())
    )]
    #[case(
        "jennings/feed_82/pl-dircopy_532/data/rudolphs_change_goes_here/data/something.txt",
        2,
        ProcessedFname::Changed("something.txt".to_string())
    )]
    fn test_shorten_target(
        #[case] fname: &str,
        #[case] times: u8,
        #[case] expected: ProcessedFname,
    ) {
        let fname = ProcessedFname::new(fname.into());
        assert_eq!(fname.shorten(times), expected)
    }

    #[rstest]
    #[case(
        "https://example.com/api/v1/uploadedfiles/search/?fname_icontains=.nii",
        "chris/uploads/brain.nii",
        ".",
        0,
        "./chris/uploads/brain.nii"
    )]
    #[case(
        "https://example.com/api/v1/uploadedfiles/search/?fname_icontains=.nii",
        "chris/uploads/brain.nii",
        "output",
        0,
        "output/chris/uploads/brain.nii"
    )]
    #[case(
        "chris/feed_1/pl-dircopy/data",
        "chris/feed_1/pl-dircopy/data/brain.nii",
        "output",
        1,
        "output/brain.nii"
    )]
    #[case(
        "chris/feed_1",
        "chris/feed_1/pl-dircopy/data/brain.nii",
        ".",
        0,
        "./pl-dircopy/data/brain.nii"
    )]
    #[case(
        "chris/feed_1",
        "chris/feed_1/pl-dircopy/data/brain.nii",
        ".",
        1,
        "./brain.nii"
    )]
    #[tokio::test]
    async fn test_where_to_download(
        #[case] src: &str,
        #[case] fname: &str,
        #[case] dst: &str,
        #[case] shorten: u8,
        #[case] expected: &str,
        example_address: &CUBEApiUrl,
    ) {
        let output_dir = Path::new(dst);
        let options = FolderOutputOptions::new(example_address, src, output_dir, shorten);
        let fname = fname.into();
        let namer = &mut Default::default();
        assert_eq!(
            options.where_to_download(&fname, namer).await,
            PathBuf::from(expected)
        )
    }
}
