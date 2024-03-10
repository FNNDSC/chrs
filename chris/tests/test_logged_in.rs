use camino::Utf8PathBuf;
use fake::Fake;
use futures::{StreamExt, TryStreamExt};
use rstest::*;

use chris::errors::FileIOError;
use chris::types::{CubeUrl, FileBrowserPath, Username};
use chris::{Account, BaseChrisClient, ChrisClient, Downloadable, FileUploadResponse, PluginRw};
use helpers::{AnyResult, TESTING_URL};

mod helpers;
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
    CubeUrl::from_static(TESTING_URL)
}

#[fixture]
#[once]
fn chris_client(cube_url: CubeUrl) -> ChrisClient {
    let username: String = fake::faker::internet::en::Username().fake();
    let email: String = fake::faker::internet::en::SafeEmail().fake();
    let password = format!("{}1234", &username.chars().rev().collect::<String>());
    let username = Username::new(username);
    futures::executor::block_on(async {
        let token = {
            let account_creator = Account::new(&cube_url, &username, &password);
            account_creator.create_account(&email).await.unwrap();
            account_creator.get_token().await.unwrap()
        };
        ChrisClient::build(cube_url, username, token)
            .unwrap()
            .connect()
            .await
            .unwrap()
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
    let files: Vec<_> = entry.iter_files(None).stream().try_collect().await?;
    let mut found = files
        .into_iter()
        .filter(|f| f.fname().as_str().ends_with("logo_chris.png"));
    assert!(found.next().is_some());
    Ok(())
}

#[fixture]
#[once]
fn pl_mri10yr(chris_client: &ChrisClient) -> PluginRw {
    let search = chris_client
        .plugin()
        .name_exact("pl-mri10yr06mo01da_normal")
        .version("1.1.4")
        .search();
    futures::executor::block_on(search.get_only()).unwrap()
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_count_and_lazy_feed_set_name(
    chris_client: &ChrisClient,
    pl_mri10yr: &PluginRw,
) -> AnyResult {
    let feed_name = uuid::Uuid::new_v4().hyphenated().to_string();
    assert_eq!(count_feeds_with_name(chris_client, &feed_name).await, 0);
    let plinst = pl_mri10yr.create_instance::<[&str]>(&[]).await.unwrap();
    let lazy_feed = plinst.feed();
    let changed_feed = lazy_feed.set_name(&feed_name).await.unwrap();
    assert_eq!(plinst.object.feed_id, changed_feed.object.id);
    assert_eq!(count_feeds_with_name(chris_client, &feed_name).await, 1);
    Ok(())
}

async fn count_feeds_with_name(client: &ChrisClient, name: &str) -> u32 {
    client
        .feeds()
        .name_exact(name)
        .search()
        .get_count()
        .await
        .unwrap()
}
