use camino::Utf8PathBuf;
use chris::errors::FileIOError;
use chris::{Account, BaseChrisClient, ChrisClient, Downloadable, FileUploadResponse};
use futures::{StreamExt, TryStreamExt};
use names::Generator;
use rstest::*;

use chris::types::{CubeUrl, FileBrowserPath, Username};

mod helpers;
use helpers::{AnyResult, TESTING_URL};

// ========================================
//            SAMPLE FILE HELPERS
// ========================================

const UPLOAD_SUBDIR: &str = "sample_files";

#[fixture]
fn sample_files_dir() -> Utf8PathBuf {
    let p = Utf8PathBuf::from("tests/data/sample_files");
    assert!(p.is_dir());
    p
}

#[fixture]
#[once]
fn sample_files(sample_files_dir: Utf8PathBuf) -> Vec<Utf8PathBuf> {
    let mut files = vec![];
    add_all_files_to(sample_files_dir, &mut files);
    files
}

/// Recursively add all files under a path to a Vec.
fn add_all_files_to(path: Utf8PathBuf, v: &mut Vec<Utf8PathBuf>) {
    if path.is_file() {
        v.push(path);
        return;
    }
    let readdir = fs_err::read_dir(path).unwrap().map(|e| e.unwrap());
    for subpath in readdir {
        add_all_files_to(Utf8PathBuf::from_path_buf(subpath.path()).unwrap(), v)
    }
}

#[fixture]
fn sample_upload_path(chris_client: &ChrisClient) -> FileBrowserPath {
    let p = format!("{}/uploads/{}", chris_client.username(), UPLOAD_SUBDIR);
    FileBrowserPath::new(p)
}

struct FileToUpload<'a>(&'a Utf8PathBuf, String);

impl FileToUpload<'_> {
    async fn upload_using(self, client: &ChrisClient) -> Result<FileUploadResponse, FileIOError> {
        client.upload_file(&self.0.as_path(), &self.1).await
    }
}

// ========================================
//            CREATE ChRIS CLIENT
// ========================================

#[fixture]
fn cube_url() -> CubeUrl {
    TESTING_URL.to_string().parse().unwrap()
}

#[fixture]
#[once]
fn chris_client(cube_url: CubeUrl) -> ChrisClient {
    let username = Generator::default().next().unwrap();
    let email = format!("{}@example.org", &username);
    let password = format!("{}1234", &username.chars().rev().collect::<String>());
    let username = Username::new(username);
    let account_creator = Account::new(cube_url, username, password);
    futures::executor::block_on(async {
        account_creator.create_account(&email).await.unwrap();
        account_creator.into_client().await.unwrap()
    })
}

#[fixture]
#[once]
fn example_uploaded_files(
    chris_client: &ChrisClient,
    sample_files: &Vec<Utf8PathBuf>,
    sample_files_dir: Utf8PathBuf,
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
    futures::executor::block_on(upload.try_collect()).unwrap()
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_filebrowser_browse_uploads(
    chris_client: &ChrisClient,
    sample_upload_path: FileBrowserPath,
    _example_uploaded_files: &Vec<FileUploadResponse>,
) -> AnyResult {
    let filebrowser = chris_client.filebrowser();
    let req = filebrowser.readdir(&sample_upload_path).await?;
    let entry = req.unwrap();
    assert!(entry.subfolders().contains(&"a_folder".to_string()));
    let files: Vec<_> = entry.iter_files().stream().try_collect().await?;
    let mut found = files
        .into_iter()
        .filter(|f| f.fname().as_str().ends_with("logo_chris.png"));
    assert!(found.next().is_some());
    Ok(())
}
