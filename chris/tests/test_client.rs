use chris::constants::*;
use chris::errors::FileIOError;
use chris::filebrowser::FileBrowserPath;
use chris::models::{data::*, *};

use chris::CUBEAuth;
use chris::ChrisClient;
use fs_err::tokio::File;
use futures::{StreamExt, TryStreamExt};
use names::Generator;
use rstest::*;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;

// ========================================
//                 CONSTANTS
// ========================================

const UPLOAD_SUBDIR: &str = "sample_files";

#[fixture]
fn cube_url() -> CUBEApiUrl {
    "http://localhost:8000/api/v1/".to_string().parse().unwrap()
}

#[fixture]
fn sample_files_dir() -> PathBuf {
    let p = PathBuf::from("tests/data/sample_files");
    assert!(p.is_dir());
    p
}

type AnyResult = Result<(), Box<dyn std::error::Error>>;

// ========================================
//                 HELPERS
// ========================================

/// Block on an async function inside a non-async context, using the current Tokio runtime.
/// Useful for implementing "async once" fixtures with [rstest](https://lib.rs/crates/rstest).
///
/// https://github.com/la10736/rstest/issues/141#issuecomment-1372784171
///
/// TODO figure out how to make a procedural attribute macro out of this.
macro_rules! block_on {
    ($async_expr:expr) => {{
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on($async_expr)
        })
    }};
}

/// Recursively add all files under a path to a Vec.
fn add_all_files_to(path: PathBuf, v: &mut Vec<PathBuf>) {
    if path.is_file() {
        v.push(path);
        return;
    }
    let readdir = fs_err::read_dir(path).unwrap().map(|e| e.unwrap());
    for subpath in readdir {
        add_all_files_to(subpath.path(), v)
    }
}

// ========================================
//                 FIXTURES
// ========================================

#[fixture]
#[once]
fn chris_client(cube_url: CUBEApiUrl) -> ChrisClient {
    let username = Generator::default().next().unwrap();
    let email = format!("{}@example.org", &username);
    let password = format!("{}1234", &username.chars().rev().collect::<String>());
    let username = Username::new(username);
    let account_creator = CUBEAuth::new(cube_url, username, password);
    block_on!(async {
        account_creator.create_account(&email).await.unwrap();
        account_creator.into_client().await.unwrap()
    })
}

#[fixture]
fn sample_upload_path(chris_client: &ChrisClient) -> FileBrowserPath {
    let p = format!("{}/uploads/{}", chris_client.username(), UPLOAD_SUBDIR);
    FileBrowserPath::new(p)
}

#[fixture]
#[once]
fn sample_files(sample_files_dir: PathBuf) -> Vec<PathBuf> {
    let mut files = vec![];
    add_all_files_to(sample_files_dir, &mut files);
    files
}

#[fixture]
#[once]
fn example_uploaded_files(
    chris_client: &ChrisClient,
    sample_files: &Vec<PathBuf>,
    sample_files_dir: PathBuf,
) -> Vec<FileUploadResponse> {
    let upload_pairs: Vec<FileToUpload> = sample_files
        .iter()
        .map(|f| {
            let rel_name = pathdiff::diff_paths(f, &sample_files_dir).unwrap();
            let dst = format!(
                "{}/{}",
                UPLOAD_SUBDIR,
                rel_name.to_string_lossy().to_string()
            );
            FileToUpload(f, dst)
        })
        .collect();
    let upload = futures::stream::iter(upload_pairs.into_iter())
        .map(|f| f.upload_using(&chris_client))
        .buffer_unordered(4);
    block_on!(upload.try_collect()).unwrap()
}

struct FileToUpload<'a>(&'a PathBuf, String);

impl FileToUpload<'_> {
    async fn upload_using(self, client: &ChrisClient) -> Result<FileUploadResponse, FileIOError> {
        client.upload_file(&self.0, &self.1).await
    }
}

// ========================================
//                 TESTS
// ========================================

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_filebrowser_browse_uploads(
    chris_client: &ChrisClient,
    sample_upload_path: FileBrowserPath,
    _example_uploaded_files: &Vec<FileUploadResponse>,
) {
    let filebrowser = chris_client.file_browser();
    let req = filebrowser.readdir(&sample_upload_path).await.unwrap();
    let entry = req.unwrap();
    assert!(entry.subfolders().contains(&"a_folder".to_string()));
    let files: Vec<DownloadableFile> = entry.iter_files().stream().try_collect().await.unwrap();
    let mut found = files
        .into_iter()
        .filter(|f| f.fname().as_str().ends_with("logo_chris.png"));
    assert!(found.next().is_some());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_plugin(chris_client: &ChrisClient) -> AnyResult {
    let plugin = chris_client
        .get_plugin(&DIRCOPY_NAME, &DIRCOPY_VERSION)
        .await?
        .unwrap();
    assert_eq!(&plugin.data.name, &*DIRCOPY_NAME);
    assert_eq!(&plugin.data.version, &*DIRCOPY_VERSION);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_download(chris_client: &ChrisClient) -> AnyResult {
    let data = b"finally some good food";
    let tmp_path = TempDir::new()?.into_path();
    let input_file = tmp_path.join(Path::new("hello.txt"));
    let output_file = tmp_path.join(Path::new("same.txt"));
    {
        File::create(&input_file).await?.write_all(data).await?;
    }
    let upload = chris_client
        .upload_file(&input_file, "test_files_upload_iter.txt")
        .await?;

    let search = chris_client.search_all_files_under(upload.fname());
    let found_file = search.get_only().await?;
    found_file.download(&output_file, false).await.unwrap();
    let downloaded = fs_err::tokio::read(output_file).await?;
    assert_eq!(data, downloaded.as_slice());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_search_files_raw(
    chris_client: &ChrisClient,
    _example_uploaded_files: &Vec<FileUploadResponse>,
) -> AnyResult {
    let cube = chris_client.url().as_str();
    let url = format!("{cube}uploadedfiles/search/?fname_nslashes=4");

    let search = chris_client.search_files_raw(url);
    assert!(search.get_count().await? > 0);

    let all_results: Vec<String> = search
        .stream()
        .map_ok(|f| f.fname().to_string())
        .try_collect()
        .await?;
    let expected_fname = "pikachu_testpass.txt";
    let mut filter = all_results.iter().filter(|f| f.ends_with(expected_fname));
    if filter.next().is_none() {
        panic!("Expected \"{}\" to be in {:?}", expected_fname, all_results)
    }

    Ok(())
}
