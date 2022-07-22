use futures::{pin_mut, Stream, TryStreamExt};
use std::path::Path;

use super::errors::{check, CUBEError, FileIOError};
use super::filebrowser::FileBrowser;
use crate::api::*;
use crate::client::pipeline::Pipeline;
use crate::client::plugin::Plugin;
use crate::client::plugininstance::PluginInstance;
use crate::common_types::{CUBEApiUrl, Username};
use crate::constants::{DIRCOPY_NAME, DIRCOPY_VERSION};
use crate::errors::{DircopyError, GetError};
use crate::pagination::*;
use crate::pipeline::CanonPipeline;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::multipart::{Form, Part};
use reqwest::Body;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::fs::{self, File, OpenOptions};
use tokio_util::codec::{BytesCodec, FramedRead};
use tokio_util::io::StreamReader;

/// _ChRIS_ client object.
#[derive(Debug)]
pub struct ChrisClient {
    client: reqwest::Client,
    url: CUBEApiUrl,
    username: Username,
    links: CUBELinks,
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
        let base_response: BaseResponse = check(res).await?.json().await?;
        Ok(ChrisClient {
            client,
            url,
            username,
            links: base_response.collection_links,
        })
    }

    /// Get the URL this client is connected to.
    pub fn url(&self) -> &CUBEApiUrl {
        &self.url
    }

    /// Get the username of this client.
    pub fn username(&self) -> &Username {
        &self.username
    }

    /// Get a client for the file browser API.
    pub fn file_browser(&self) -> FileBrowser {
        FileBrowser::new(self.client.clone(), self.links.filebrowser.clone())
    }

    /// Upload a pipeline to _ChRIS_.
    pub async fn upload_pipeline(
        &self,
        pipeline: &CanonPipeline,
    ) -> Result<PipelineResponse, CUBEError> {
        let res = self
            .client
            .post(&self.links.pipelines.to_string())
            .json(pipeline)
            .send()
            .await?;
        Ok(check(res).await?.json().await?)
    }

    /// Iterate over files in the given query.
    ///
    /// Usage: <https://docs.rs/async-stream/0.3.3/async_stream/#usage>
    pub fn iter_files<'a>(
        &'a self,
        url: &'a AnyFilesUrl,
    ) -> impl Stream<Item = Result<DownloadableFile, reqwest::Error>> + 'a {
        self.paginate(url)
    }

    /// Fetch the count from a paginated resource.
    pub async fn get_count(&self, url: &str) -> Result<u32, CUBEError> {
        let res = self.client.get(url).query(&LIMIT_ZERO).send().await?;
        let data: HasCount = check(res).await?.json().await?;
        Ok(data.count)
    }

    /// Upload a file to ChRIS. `upload_path` is a fname relative to `"<username>/uploads/"`.
    pub async fn upload_file(
        &self,
        local_file: &Path,
        upload_path: &str,
    ) -> Result<FileUploadResponse, FileIOError> {
        let swift_path = format!("{}/uploads/{}", self.username, upload_path);

        // https://github.com/seanmonstar/reqwest/issues/646#issuecomment-616985015
        let filename = local_file
            .file_name()
            .ok_or_else(|| FileIOError::PathError(local_file.to_string_lossy().to_string()))?
            .to_string_lossy()
            .to_string();
        let file = File::open(local_file).await.map_err(FileIOError::IO)?;
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
        Ok(check(res).await?.json().await?)
    }

    /// Download a file from _ChRIS_ to a local path.
    pub async fn download_file(
        &self,
        src: &impl Downloadable,
        dst: &Path,
        clobber: bool,
    ) -> Result<(), FileIOError> {
        let mut file = if clobber {
            File::create(dst).await
        } else {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dst)
                .await
        }
        .map_err(FileIOError::IO)?;
        let res = self.client.get(src.file_resource().as_str()).send().await?;
        let stream = res
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::ConnectionAborted, e));
        let mut reader = StreamReader::new(stream);
        tokio::io::copy(&mut reader, &mut file).await?;
        Ok(())
    }

    fn paginate<'a, U: 'a + PaginatedUrl, R: 'a + DeserializeOwned>(
        &'a self,
        url: &'a U,
    ) -> impl Stream<Item = Result<R, reqwest::Error>> + 'a {
        // TODO check the error, if it's a problem with .json,
        // tell user to check documentation for supported URLs
        paginate(&self.client, Some(url))
    }

    /// Get a specific plugin by (name_exact, version).
    pub async fn get_plugin(
        &self,
        name_exact: &PluginName,
        version: &PluginVersion,
    ) -> Result<Option<PluginResponse>, CUBEError> {
        let query = &[
            ("name_exact", name_exact.as_str()),
            ("version", version.as_str()),
        ];
        // invariant: only one search result will be found by (name_exact, version)
        let plugin = self.get_first(&self.links.plugins, query).await?;
        Ok(plugin)
    }

    /// Get the first object from a search.
    async fn get_first<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        base_url: &impl PaginatedUrl,
        query: &T,
    ) -> Result<Option<R>, reqwest::Error> {
        let url = SearchUrl::of(base_url, query).unwrap();
        let results = self.paginate(&url);
        pin_mut!(results);
        results.try_next().await
    }

    /// Create a plugin instance of (i.e. run) `pl-dircopy`
    pub async fn dircopy(&self, dir: &str) -> Result<PluginInstance, DircopyError> {
        let plugin_response = self
            .get_plugin(&DIRCOPY_NAME, &DIRCOPY_VERSION)
            .await
            .map_err(DircopyError::CUBEError)?
            .ok_or(DircopyError::DircopyNotFound(
                &DIRCOPY_NAME,
                &DIRCOPY_VERSION,
            ))?;
        let plugin = Plugin::new(self.client.clone(), plugin_response);
        Ok(plugin.create_instance(&DircopyPayload { dir }).await?)
    }

    pub async fn get_pipeline(&self, name: &str) -> Result<Option<Pipeline>, GetError> {
        let query = &[("name", name)];
        let pipeline = self
            .get_first(&self.links.pipelines, query)
            .await
            .map_err(CUBEError::from)?;
        Ok(pipeline.map(|p| Pipeline::new(self.client.clone(), p)))
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

#[derive(Deserialize)]
struct HasCount {
    count: u32,
}

#[derive(Serialize)]
struct DircopyPayload<'a> {
    dir: &'a str,
}

// ============================== TESTS ==============================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ParameterName, ParameterValue, PluginName, PluginVersion};
    use crate::auth::CUBEAuth;
    use crate::pipeline::canon::{ExpandedTreeParameter, ExpandedTreePipeline, ExpandedTreePiping};
    use futures::future::{join_all, try_join_all};
    use futures::{pin_mut, StreamExt};
    use names::Generator;
    use rstest::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    const CUBE_URL: &str = "http://localhost:8000/api/v1/";

    type AnyResult = Result<(), Box<dyn std::error::Error>>;

    #[rstest]
    #[tokio::test]
    async fn test_download(#[future] future_client: ChrisClient) -> AnyResult {
        let chris: ChrisClient = future_client.await;
        let data = b"finally some good content";
        let tmp_path = TempDir::new()?.into_path();
        let input_file = tmp_path.join(Path::new("hello.txt"));
        let output_file = tmp_path.join(Path::new("same.txt"));
        {
            fs::File::create(&input_file).await?.write_all(data).await?;
        }
        let upload = chris
            .upload_file(&input_file, "test_files_upload_iter.txt")
            .await?;
        chris.download_file(&upload, &output_file, false).await?;
        let downloaded = tokio::fs::read(output_file).await?;
        assert_eq!(data, downloaded.as_slice());
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_files_upload_iter(#[future] future_client: ChrisClient) -> AnyResult {
        // ---------- create some test files ----------
        let num_files = 42;
        let chris: ChrisClient = future_client.await;
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
            .map(|(f, upload_path)| chris.upload_file(f, upload_path));

        // ---------- check for errors ----------
        let results = join_all(future_uploads).await;
        let failures: Vec<&FileIOError> = results.iter().filter_map(|r| r.as_ref().err()).collect();
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
            chris.username, &parent
        ));

        assert_eq!(num_files, chris.get_count(search.as_str()).await?);

        // ---------- test pagination ----------
        // Get the fnames of all the file we just uploaded,
        // and make sure that they match the upload paths we specified.
        let s = chris.iter_files(&search);
        pin_mut!(s);
        while let Some(f) = s.next().await {
            let file = f?;
            let mut removed: Option<String> = None;
            for (i, upload_path) in upload_paths.iter().enumerate() {
                if file.fname().as_str().ends_with(upload_path) {
                    removed = Some(upload_paths.swap_remove(i));
                    break;
                }
            }
            assert!(
                removed.is_some(),
                "fname=\"{}\" not found in: {:?}",
                file.fname(),
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

    #[fixture]
    async fn future_client() -> ChrisClient {
        // due to a limitation of rstest, we cannot have a once setup async fixture
        // run before every other unit test, so we have to create a new ChRIS account
        // for each unit test.
        //
        // https://github.com/la10736/rstest/issues/141
        let username = Generator::default().next().unwrap();
        let email = format!("{}@example.org", &username);
        let account_creator = CUBEAuth {
            password: format!(
                "{}1234",
                username.as_str().chars().rev().collect::<String>()
            ),
            username: Username::new(username),
            url: CUBEApiUrl::try_from(CUBE_URL).unwrap(),
            client: &reqwest::Client::new(),
        };
        account_creator.create_account(&email).await.unwrap();
        account_creator.into_client().await.unwrap()
    }

    #[fixture]
    fn example_pipeline() -> ExpandedTreePipeline {
        let mut name_generator = Generator::default();
        ExpandedTreePipeline {
            authors: name_generator.next().unwrap(),
            name: name_generator.next().unwrap(),
            description: name_generator.next().unwrap(),
            category: "Chrs Test".to_string(),
            locked: false,
            plugin_tree: vec![
                ExpandedTreePiping {
                    plugin_name: PluginName::from("pl-simpledsapp"),
                    plugin_version: PluginVersion::from("2.1.0"),
                    previous_index: None,
                    plugin_parameter_defaults: Some(vec![ExpandedTreeParameter {
                        name: ParameterName::from("prefix"),
                        default: ParameterValue::Str("chrs-test-".to_string()),
                    }]),
                },
                ExpandedTreePiping {
                    plugin_name: PluginName::from("pl-simpledsapp"),
                    plugin_version: PluginVersion::from("2.1.0"),
                    previous_index: Some(0),
                    plugin_parameter_defaults: None,
                },
            ],
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_plugin(#[future] future_client: ChrisClient) -> AnyResult {
        let chris: ChrisClient = future_client.await;
        let plugin = chris
            .get_plugin(&DIRCOPY_NAME, &DIRCOPY_VERSION)
            .await?
            .unwrap();
        assert_eq!(&plugin.name, &*DIRCOPY_NAME);
        assert_eq!(&plugin.version, &*DIRCOPY_VERSION);
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_e2e(
        example_pipeline: ExpandedTreePipeline,
        #[future] future_client: ChrisClient,
    ) -> AnyResult {
        let chris: ChrisClient = future_client.await;

        ////////////////
        // upload file
        let data = b"finally some good content";
        let tmp_path = TempDir::new()?.into_path();
        let input_file = tmp_path.join(Path::new("hello.txt"));
        {
            fs::File::create(&input_file).await?.write_all(data).await?;
        }
        let upload_path = format!(
            "{}/{}",
            "test-chrs-upload",
            Generator::default().next().unwrap()
        );
        let upload = chris.upload_file(&input_file, upload_path.as_str()).await?;

        ////////////////
        // run pl-dircopy
        let dircopy_instance = chris.dircopy(upload.fname().as_str()).await?;
        assert_eq!(
            &dircopy_instance.plugin_instance.plugin_name,
            &*DIRCOPY_NAME
        );

        ////////////////
        // name created feed
        let feed = dircopy_instance.get_feed();
        let feed_details = feed.set_name("a new name").await?;
        assert_eq!(feed_details.name.as_str(), "a new name");

        ////////////////
        // upload pipeline
        let num_pipings = example_pipeline.plugin_tree.len();
        let pipeline_name = example_pipeline.name.clone();
        let uploaded_pipeline = chris.upload_pipeline(&example_pipeline.into()).await?;
        assert_eq!(&uploaded_pipeline.name, &pipeline_name);
        let gotten = chris.get_pipeline(&pipeline_name).await?.expect(
            format!(
                "Just uploaded the pipeline \"{}\" but cannot find it",
                &pipeline_name
            )
            .as_str(),
        );
        assert_eq!(&gotten.pipeline.name, &pipeline_name);

        ////////////////
        // create workflow
        let workflow = gotten
            .create_workflow(dircopy_instance.plugin_instance.id)
            .await?;
        assert_eq!(&workflow.pipeline_name, &pipeline_name);
        let n_created_plugin_inst = workflow.created_plugin_inst_ids.split(',').count();
        assert_eq!(n_created_plugin_inst, num_pipings);

        Ok(())
    }
}
