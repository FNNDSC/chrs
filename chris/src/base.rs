// use serde::Deserialize;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use crate::api::*;
use crate::common_types::{CUBEApiUrl, Username};
use crate::pagination::*;

#[derive(Debug)]
pub struct ChrisClient {
    client: reqwest::Client,
    pub url: CUBEApiUrl,
    pub username: Username,
    links: CUBELinks,
}

impl ChrisClient {
    pub async fn new(url: CUBEApiUrl, username: Username, token: String) -> Result<Self, reqwest::Error> {
        let client = reqwest::ClientBuilder::new()
            .default_headers(token2header(&token))
            .build()?;
        let res = client.get(url.as_str()).query(&LIMIT_ZERO).send().await?;
        let links: CUBELinks = res.json().await?;
        Ok(ChrisClient { client, url, username, links })
    }
}

const LIMIT_ZERO: PaginationQuery = PaginationQuery {limit: 0, offset: 0};

fn token2header(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let auth_data = format!("token {}", token);
    let mut value: HeaderValue = auth_data.parse().unwrap();
    value.set_sensitive(true);
    headers.insert(AUTHORIZATION, value);
    headers.insert(ACCEPT, "application/json".parse().unwrap());
    headers
}


// #[cfg(test)]
// mod tests {
//     use std::str::FromStr;
//     use super::*;
//     use rstest::*;
//     use names::Generator;
//     use crate::auth::CUBEAuth;
//
//     const CUBE_URL: &str = "http://localhost:8000/api/v1/";
//
//     type AnyResult = Result<(), Box<dyn std::error::Error>>;
//
//     #[tokio::test]
//     async fn test_files() -> AnyResult {
//
//         Ok(())
//     }
//
//     #[fixture]
//     #[once]
//     #[tokio::test]
//     async fn client() {
//         let url = CUBEApiUrl::from_str(&CUBE_URL).unwrap();
//         let mut name_generator = Generator::default();
//         let username_value = name_generator.next().unwrap();
//         let username = Username::from_str(username_value.as_str())?;
//         let email = format!("{}@example.org", &username);
//         let account_creator = CUBEAuth {
//             username: &username,
//             password: &*format!("{}1234", username.chars().rev().collect::<String>()),
//             url: &url,
//             client: &reqwest::Client::new(),
//         };
//         let created_account = account_creator.create_account(&email).await?;
//         let token = account_creator.get_token().await?;
//         ChrisClient::new(url,  username, token)
//     }
// }
