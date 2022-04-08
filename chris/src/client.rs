use futures::Stream;
use std::path::Path;

use crate::api::*;
use crate::common_types::{CUBEApiUrl, Username};
use crate::pagination::*;
use crate::pipeline::CanonPipeline;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::multipart::{Form, Part};
use reqwest::{Body, Error};
use serde::de::DeserializeOwned;
use tokio::fs::{self, File};
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Debug)]
pub struct ChrisClient {
    client: reqwest::Client,
    pub url: CUBEApiUrl,
    pub username: Username,
    links: CUBELinks,
    pub friendly_error: bool,
}

impl ChrisClient {
    pub async fn new(
        url: CUBEApiUrl,
        username: Username,
        token: String,
    ) -> Result<Self, CUBEError> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(token2header(&token))
            .build()?;
        let res = client.get(url.as_str()).query(&LIMIT_ZERO).send().await?;
        res.error_for_status_ref()?;
        let base_response: BaseResponse = res.json().await?;
        Ok(ChrisClient {
            client,
            url,
            username,
            links: base_response.collection_links,
            friendly_error: true,
        })
    }

    pub async fn upload_pipeline(
        &self,
        pipeline: &CanonPipeline,
    ) -> Result<PipelineUploadResponse, CUBEError> {
        let res = self
            .client
            .post(&self.links.pipelines.to_string())
            .json(pipeline)
            .send()
            .await?;
        Ok(self.check_error(res).await?.json().await?)
    }

    /// Iterate over files in the given query.
    ///
    /// Usage: https://docs.rs/async-stream/0.3.3/async_stream/#usage
    pub fn iter_files<'a>(
        &'a self,
        url: &'a AnyFilesUrl,
    ) -> impl Stream<Item = Result<DownloadableFile, reqwest::Error>> + 'a {
        self.paginate(url)
    }

    /// Fetch the count of number of files in the given query.
    pub async fn count_files(&self, url: &AnyFilesUrl) -> Result<u32, CUBEError> {
        let s: &str = url.as_ref();
        let res = self.client.get(s).query(&LIMIT_ZERO).send().await?;
        let data: Paginated<AnyFilesUrl, DownloadableFile> =
            self.check_error(res).await?.json().await?;
        Ok(data.count)
    }

    /// Upload a file to ChRIS. `upload_path` is a fname relative to `"<username>/uploads/"`.
    pub async fn upload_file(
        &self,
        local_file: &Path,
        upload_path: &str,
    ) -> Result<FileUploadResponse, UploadError> {
        let swift_path = format!("{}/uploads/{}", self.username, upload_path);

        // https://github.com/seanmonstar/reqwest/issues/646#issuecomment-616985015
        let filename = local_file
            .file_name()
            .ok_or_else(|| UploadError::PathError(local_file.to_string_lossy().to_string()))?
            .to_string_lossy()
            .to_string();
        let file = File::open(local_file).await.map_err(UploadError::IO)?;
        let content_length = fs::metadata(local_file).await?.len();
        let reader = Body::wrap_stream(FramedRead::new(file, BytesCodec::new()));

        let form = Form::new().text("upload_path", swift_path).part(
            "fname",
            Part::stream_with_length(reader, content_length).file_name(filename),
        );
        let req = self
            .client
            .post(self.links.uploadedfiles.as_str())
            .multipart(form);
        let res = req.send().await?;
        Ok(self.check_error(res).await?.json().await?)
    }

    // ============================== HELPERS ==============================

    async fn check_error(&self, res: reqwest::Response) -> Result<reqwest::Response, CUBEError> {
        check_error_helper(res, self.friendly_error).await
    }

    fn paginate<'a, U: 'a + PaginatedUrl, R: 'a + DeserializeOwned>(
        &'a self,
        url: &'a U,
    ) -> impl Stream<Item = Result<R, reqwest::Error>> + 'a {
        // TODO check the error, if it's a problem with .json,
        // tell user to check documentation for supported URLs
        paginate(&self.client, url)
    }
}

/// If `friendly_error == true` and `res` has an error status,
/// get the text from the response and produce a [CUBEError::Friendly]
async fn check_error_helper(
    res: reqwest::Response,
    friendly_error: bool,
) -> Result<reqwest::Response, CUBEError> {
    match res.error_for_status_ref() {
        Ok(_) => Ok(res),
        Err(e) => {
            if friendly_error {
                let status = res.status();
                let reason = status.canonical_reason().unwrap_or("unknown reason");
                let body = res.text().await.map_err(CUBEError::Raw)?;
                let msg = format!("({:?} {:?}): {}", status, reason, body);
                Err(CUBEError::Friendly(msg))
            } else {
                Err(CUBEError::Raw(e))
            }
        }
    }
}

// ============================== ERRORS ==============================

/// An error type which, in the case of HTTP status 400-599, can optionally
/// include the body as text in the Display message.
#[derive(thiserror::Error, Debug)]
pub enum CUBEError {
    #[error("{0}")]
    Friendly(String),
    #[error(transparent)]
    Raw(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("\"{0}\" is an invalid file path")]
    PathError(String),
    #[error(transparent)]
    Cube(CUBEError),
    #[error(transparent)]
    IO(std::io::Error),
}

impl From<reqwest::Error> for UploadError {
    fn from(e: Error) -> Self {
        UploadError::Cube(CUBEError::Raw(e))
    }
}

impl From<CUBEError> for UploadError {
    fn from(e: CUBEError) -> Self {
        UploadError::Cube(e)
    }
}

impl From<std::io::Error> for UploadError {
    fn from(e: std::io::Error) -> Self {
        UploadError::IO(e)
    }
}

// ============================== HELPERS ==============================

const LIMIT_ZERO: PaginationQuery = PaginationQuery {
    limit: 0,
    offset: 0,
};

fn token2header(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let auth_data = format!("token {}", token);
    let mut value: HeaderValue = auth_data.parse().unwrap();
    value.set_sensitive(true);
    headers.insert(AUTHORIZATION, value);
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers
}

// ============================== TESTS ==============================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ParameterName, ParameterValue, PluginName, PluginVersion};
    use crate::auth::CUBEAuth;
    use crate::pipeline::{ExpandedTreeParameter, ExpandedTreePipeline, ExpandedTreePiping};
    use futures::future::{join_all, try_join_all};
    use futures::{pin_mut, StreamExt};
    use names::Generator;
    use rstest::*;
    use std::path::PathBuf;
    use std::str::FromStr;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    const CUBE_URL: &str = "http://localhost:8000/api/v1/";

    type AnyResult = Result<(), Box<dyn std::error::Error>>;

    #[rstest]
    #[tokio::test]
    async fn test_files_upload_iter(#[future] future_client: ChrisClient) -> AnyResult {
        // ---------- create some test files ----------
        let num_files = 42;
        let client: ChrisClient = future_client.await;
        let tmp_dir = TempDir::new()?;
        let tmp_path = tmp_dir.path();
        let file_names: Vec<PathBuf> = (0..num_files)
            .map(|i| format!("{}", i))
            .map(PathBuf::from)
            .map(|p| tmp_path.join(p))
            .collect();
        let writes = file_names
            .iter()
            .map(|f| fs::File::create(f))
            .zip(Generator::default())
            .map(|(f, data)| async move { f.await.unwrap().write_all(data.as_ref()).await });
        try_join_all(writes).await?;

        // ---------- upload files to ChRIS ----------
        let parent = Generator::default().next().unwrap();
        let mut upload_paths: Vec<String> = file_names
            .iter()
            .map(|f| format!("{}/{}", &parent, f.file_name().unwrap().to_string_lossy()))
            .collect();
        let future_uploads = file_names
            .iter()
            .zip(upload_paths.as_slice())
            .map(|(f, upload_path)| client.upload_file(f, upload_path));

        // ---------- check for errors ----------
        let results = join_all(future_uploads).await;
        let failures: Vec<&UploadError> = results.iter().filter_map(|r| r.as_ref().err()).collect();
        assert_eq!(
            0,
            failures.len(),
            "{}/{} uploads failed. See errors below. \
            In case of 500 \"Internal Server Error,\" this is because the backend cannot \
            keep up with the speed of Rust. Please try again or get a faster computer.\
            \n{:?}",
            failures.len(),
            num_files,
            failures
        );

        // ---------- test client.count_files ----------
        let search = AnyFilesUrl::new(format!(
            "http://localhost:8000/api/v1/uploadedfiles/search/\
        ?fname_icontains={}/uploads/{}&fname_nslashes=3",
            client.username, &parent
        ));

        assert_eq!(num_files, client.count_files(&search).await?);

        // ---------- test pagination ----------
        // Get the fnames of all the file we just uploaded,
        // and make sure that they match the upload paths we specified.
        let s = client.iter_files(&search);
        pin_mut!(s);
        while let Some(f) = s.next().await {
            let file = f?;
            let mut removed: Option<String> = None;
            for (i, upload_path) in upload_paths.iter().enumerate() {
                if file.fname.as_str().ends_with(upload_path) {
                    removed = Some(upload_paths.swap_remove(i));
                    break;
                }
            }
            assert!(
                removed.is_some(),
                "fname=\"{}\" not found in: {:?}",
                file.fname,
                upload_paths
            )
        }
        assert!(
            upload_paths.is_empty(),
            "These uploaded files were not found by `client.iter_files(&search)`: {:?}",
            upload_paths
        );
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_upload_pipeline(
        #[future] future_client: ChrisClient,
        example_pipeline: CanonPipeline,
    ) -> AnyResult {
        let uploaded_pipeline = future_client
            .await
            .upload_pipeline(&example_pipeline)
            .await?;
        assert_eq!(uploaded_pipeline.name, example_pipeline.name);
        Ok(())
    }

    #[fixture]
    async fn future_client() -> ChrisClient {
        // due to a limitation of rstest, we cannot have a once setup async fixture
        // run before every other unit test, so we have to create a new ChRIS account
        // for each unit test.
        //
        // https://github.com/la10736/rstest/issues/141
        let username_value = Generator::default().next().unwrap();
        let url = CUBEApiUrl::from_str(&CUBE_URL).unwrap();
        let username = Username::new(username_value);
        let email = format!("{}@example.org", &username);
        let account_creator = CUBEAuth {
            username: &username,
            password: &*format!(
                "{}1234",
                username.as_str().chars().rev().collect::<String>()
            ),
            url: &url,
            client: &reqwest::Client::new(),
        };
        account_creator.create_account(&email).await.unwrap();
        let token = account_creator.get_token().await.unwrap();
        ChrisClient::new(url, username, token).await.unwrap()
    }

    #[fixture]
    fn example_pipeline() -> CanonPipeline {
        let mut name_generator = Generator::default();
        ExpandedTreePipeline {
            authors: name_generator.next().unwrap(),
            name: name_generator.next().unwrap(),
            description: name_generator.next().unwrap(),
            category: "Chrs Test".to_string(),
            locked: false,
            plugin_tree: vec![
                ExpandedTreePiping {
                    plugin_name: PluginName::new("pl-simpledsapp"),
                    plugin_version: PluginVersion::new("2.0.2"),
                    previous_index: None,
                    plugin_parameter_defaults: Some(vec![ExpandedTreeParameter {
                        name: ParameterName::new("prefix"),
                        default: ParameterValue::Str("chrs-test-".to_string()),
                    }]),
                },
                ExpandedTreePiping {
                    plugin_name: PluginName::new("pl-simpledsapp"),
                    plugin_version: PluginVersion::new("2.0.2"),
                    previous_index: Some(0),
                    plugin_parameter_defaults: None,
                },
            ],
        }
        .into()
    }
}
