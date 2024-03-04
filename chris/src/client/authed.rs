use super::base;
use super::search::LIMIT_ZERO;
use super::searches::{FeedSearchBuilder, PluginSearchBuilder, SearchBuilder};
use super::variant::RoAccess;
use crate::errors::{check, CubeError, FileIOError};
use crate::models::{BaseResponse, CubeLinks, FileUploadResponse};
use crate::types::*;
use crate::{Access, BaseChrisClient, FeedResponse, FileBrowser, LinkedModel, RwAccess};
use bytes::Bytes;
use camino::Utf8Path;
use fs_err::tokio::File;
use futures::TryStream;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::multipart::{Form, Part};
use reqwest::Body;
use std::borrow::Cow;
use std::marker::PhantomData;
use tokio_util::codec::{BytesCodec, FramedRead};

/// _ChRIS_ user client with read-write API access.
pub type ChrisClient = AuthedChrisClient<RwAccess>;

/// Authenticated _ChRIS_ user client.
#[derive(Debug)]
pub struct AuthedChrisClient<A: Access> {
    client: reqwest_middleware::ClientWithMiddleware,
    url: CubeUrl,
    username: Username,
    links: CubeLinks,
    phantom: PhantomData<A>,
}

pub struct ChrisClientBuilder {
    url: CubeUrl,
    username: Username,
    builder: reqwest_middleware::ClientBuilder,
}

impl ChrisClientBuilder {
    pub(crate) fn new(
        url: CubeUrl,
        username: Username,
        token: &str,
    ) -> Result<Self, reqwest::Error> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(token2header(token))
            .build()?;
        let builder = reqwest_middleware::ClientBuilder::new(client);
        Ok(Self {
            url,
            username,
            builder,
        })
    }

    /// Add middleware to the HTTP client.
    pub fn with<M: reqwest_middleware::Middleware>(self, middleware: M) -> Self {
        Self {
            url: self.url,
            username: self.username,
            builder: self.builder.with(middleware),
        }
    }

    /// Connect to the ChRIS API.
    pub async fn connect(self) -> Result<ChrisClient, CubeError> {
        let client = self.builder.build();
        let res = client
            .get(self.url.as_str())
            .query(&LIMIT_ZERO)
            .send()
            .await?;
        let base_response: BaseResponse = check(res).await?.json().await?;
        Ok(ChrisClient {
            client,
            username: self.username,
            url: self.url,
            links: base_response.collection_links,
            phantom: Default::default(),
        })
    }
}

impl<A: Access> AuthedChrisClient<A> {
    /// Create a client builder.
    pub fn build(
        url: CubeUrl,
        username: Username,
        token: impl AsRef<str>,
    ) -> Result<ChrisClientBuilder, reqwest::Error> {
        ChrisClientBuilder::new(url, username, token.as_ref())
    }

    /// Get username
    pub fn username(&self) -> &Username {
        &self.username
    }

    // ==================================================
    //                 FILES UPLOAD
    // ==================================================

    /// Create a _ChRIS_ uploadedfile from a stream of bytes.
    ///
    /// [`ChrisClient::upload_stream`] is a lower-level function called by
    /// [`ChrisClient::upload_file`]. Most often, developers would be
    /// interested in the former.
    ///
    /// # Arguments
    ///
    /// - stream: stream of byte data
    /// - filename: included in the multi-part post request (not the _ChRIS_ file path)
    /// - path: _ChRIS_ file path starting with `"<username>/uploads/"`
    pub async fn upload_stream<S, F, P>(
        &self,
        stream: S,
        filename: F,
        path: P,
        content_length: u64,
    ) -> Result<FileUploadResponse, FileIOError>
    where
        S: TryStream + Send + Sync + 'static,
        S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        Bytes: From<S::Ok>,
        F: Into<Cow<'static, str>>,
        P: Into<Cow<'static, str>>,
    {
        // https://github.com/seanmonstar/reqwest/issues/646#issuecomment-616985015
        let reader = Body::wrap_stream(stream);
        let form = Form::new().text("upload_path", path).part(
            "fname",
            Part::stream_with_length(reader, content_length).file_name(filename),
        );
        let req = self
            .client
            .post(self.links.userfiles.as_str())
            .multipart(form);
        let res = req.send().await?;
        Ok(check(res).await?.json().await?)
    }

    /// Upload a file to ChRIS. `upload_path` is a fname relative to `"<username>/uploads/"`.
    pub async fn upload_file(
        &self,
        local_file: &Utf8Path,
        upload_path: &str,
    ) -> Result<FileUploadResponse, FileIOError> {
        let path = format!("{}/uploads/{}", self.username, upload_path);

        let filename = local_file
            .file_name()
            .ok_or_else(|| FileIOError::PathError(local_file.to_string()))?
            .to_string();
        let file = File::open(local_file).await.map_err(FileIOError::IO)?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let content_length = fs_err::tokio::metadata(local_file).await?.len();
        self.upload_stream(stream, filename, path, content_length)
            .await
    }
}

fn token2header(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let auth_data = format!("token {}", token);
    let mut value: HeaderValue = auth_data.parse().unwrap();
    value.set_sensitive(true);
    headers.insert(AUTHORIZATION, value);
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers
}

impl<A: Access> BaseChrisClient<A> for AuthedChrisClient<A> {
    fn filebrowser(&self) -> FileBrowser {
        FileBrowser::new(self.client.clone(), &self.links.filebrowser)
    }

    fn url(&self) -> &CubeUrl {
        &self.url
    }

    fn plugin(&self) -> PluginSearchBuilder<A> {
        SearchBuilder::new(&self.client, &self.links.plugins)
    }

    fn public_feeds(&self) -> FeedSearchBuilder<RoAccess> {
        FeedSearchBuilder::new(&self.client, &self.links.public_feeds)
    }

    async fn get_feed(&self, id: FeedId) -> Result<LinkedModel<FeedResponse, A>, CubeError> {
        base::get_feed(&self.client, self.url(), id).await
    }
}

impl ChrisClient {
    /// Convert to a [RoAccess] client.
    pub fn into_ro(self) -> AuthedChrisClient<RoAccess> {
        AuthedChrisClient::<RoAccess> {
            client: self.client,
            url: self.url,
            username: self.username,
            links: self.links,
            phantom: Default::default(),
        }
    }
}
