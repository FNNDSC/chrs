use chris::common_types::{CUBEApiUrl, Username};
use chris::errors::FileIOError;
use chris::filebrowser::FileBrowserPath;
use chris::models::*;
use chris::CUBEAuth;
use chris::ChrisClient;
use futures::{StreamExt, TryStreamExt};
use names::Generator;
use rstest::*;
use std::path::{Path, PathBuf};

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
    example_uploaded_files: &Vec<FileUploadResponse>,
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
