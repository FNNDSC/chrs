use crate::api::*;
use crate::common_types::{CUBEApiUrl, Username};
use crate::pagination::*;
use crate::pipeline::CanonPipeline;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};

#[derive(Debug)]
pub struct ChrisClient {
    client: reqwest::Client,
    pub url: CUBEApiUrl,
    pub username: Username,
    links: CUBELinks,
    pub friendly_error: bool
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
            friendly_error: true
        })
    }

    pub async fn upload_pipeline(&self, pipeline: &CanonPipeline) -> Result<PipelineUploadResponse, CUBEError> {
        let res = self.client.post(&self.links.pipelines.to_string())
            .json(pipeline).send().await?;
        Ok(self.check_error(res).await?.json().await?)
    }

    async fn check_error(&self, res: reqwest::Response) -> Result<reqwest::Response, CUBEError> {
        check_error_helper(res, self.friendly_error).await
    }
}

/// If `friendly_error == true` and `res` has an error status,
/// get the text from the response and produce a [CUBEError::Friendly]
async fn check_error_helper(res: reqwest::Response, friendly_error: bool) -> Result<reqwest::Response, CUBEError> {
    match res.error_for_status_ref() {
        Ok(_) => {Ok(res)}
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


#[derive(thiserror::Error, Debug)]
pub enum CUBEError {
    #[error("{0}")]
    Friendly(String),
    #[error(transparent)]
    Raw(#[from] reqwest::Error)
}

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

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use super::*;
    use rstest::*;
    use names::Generator;
    use crate::auth::CUBEAuth;
    use crate::pipeline::{ExpandedTreePipeline, ExpandedTreePiping, ExpandedTreeParameter};
    use crate::api::{PluginName, PluginVersion, ParameterName, ParameterValue};

    const CUBE_URL: &str = "http://localhost:8000/api/v1/";

    type AnyResult = Result<(), Box<dyn std::error::Error>>;

    #[rstest]
    #[tokio::test]
    async fn test_upload_pipeline(#[future] client: ChrisClient, example_pipeline: CanonPipeline) -> AnyResult {
        let uploaded_pipeline = client.await.upload_pipeline(&example_pipeline).await?;
        assert_eq!(uploaded_pipeline.name, example_pipeline.name);
        Ok(())
    }

    #[fixture]
    async fn client() -> ChrisClient {
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
            password: &*format!("{}1234", username.as_str().chars().rev().collect::<String>()),
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
                    plugin_parameter_defaults: Some(vec![
                        ExpandedTreeParameter {
                            name: ParameterName::new("prefix"),
                            default: ParameterValue::Str("chrs-test-".to_string())
                        }
                    ])
                },
                ExpandedTreePiping {
                    plugin_name: PluginName::new("pl-simpledsapp"),
                    plugin_version: PluginVersion::new("2.0.2"),
                    previous_index: Some(0),
                    plugin_parameter_defaults: None
                }
            ]
        }.into()
    }
}
