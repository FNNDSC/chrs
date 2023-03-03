//! Predecessors to [crate::ChrisClient] for getting _ChRIS_ authorization
//! tokens or creating _ChRIS_ accounts.

use crate::common_types::{CUBEApiUrl, Username};
use crate::errors::CUBEError;
use crate::models::{UserId, UserUrl};
use crate::ChrisClient;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AuthTokenResponse {
    // clippy doesn't know how serde works
    #[allow(dead_code)]
    token: String,
}

#[derive(Deserialize)]
pub struct UserCreatedResponse {
    pub url: UserUrl,
    pub id: UserId,
    pub username: Username,
    pub email: String,
    // feed: Vec  // idk what this is
}

#[derive(Serialize)]
struct Credentials<'a> {
    username: &'a Username,
    password: &'a str,
}

#[derive(Serialize)]
struct CreateUserData<'a> {
    username: &'a Username,
    password: &'a str,
    email: &'a str,
}

/// CUBE username and password struct.
/// [CUBEAuth] is a builder for [chris::ChrisClient].
pub struct CUBEAuth {
    pub client: reqwest::Client,
    pub url: CUBEApiUrl,
    pub username: Username,
    pub password: String,
}

impl CUBEAuth {
    pub fn new(url: CUBEApiUrl, username: Username, password: String) -> Self {
        Self {
            client: Default::default(),
            url,
            username,
            password,
        }
    }

    pub async fn get_token(&self) -> Result<String, reqwest::Error> {
        let auth_url = format!("{}auth-token/", &self.url);
        let req = self
            .client
            .post(auth_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&Credentials {
                username: &self.username,
                password: &self.password,
            });
        let res = req.send().await?;
        res.error_for_status_ref()?;
        let token_object: AuthTokenResponse = res.json().await?;
        Ok(token_object.token)
    }

    pub async fn create_account(&self, email: &str) -> Result<UserCreatedResponse, reqwest::Error> {
        let users_url = format!("{}users/", &self.url);
        let req = self
            .client
            .post(users_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&CreateUserData {
                username: &self.username,
                password: &self.password,
                email,
            });
        let res = req.send().await?;
        res.error_for_status_ref()?;
        let created_user: UserCreatedResponse = res.json().await?;
        Ok(created_user)
    }

    pub async fn into_client(self) -> Result<ChrisClient, CUBEError> {
        let token = self.get_token().await?;
        ChrisClient::new(self.url, self.username, token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use names::Generator;
    use rstest::*;

    const CUBE_URL: &str = "http://localhost:8000/api/v1/";

    lazy_static! {
        static ref CLIENT: reqwest::Client = reqwest::Client::new();
    }

    #[fixture]
    fn client() -> reqwest::Client {
        CLIENT.clone()
    }

    #[fixture]
    fn cube_url() -> CUBEApiUrl {
        CUBEApiUrl::try_from(CUBE_URL).unwrap()
    }

    #[rstest]
    #[tokio::test]
    async fn test_get_token(cube_url: CUBEApiUrl, client: reqwest::Client) {
        let account = CUBEAuth {
            username: Username::new("chris".to_string()),
            password: "chris1234".to_string(),
            url: cube_url,
            client: client.clone(),
        };

        let token = account.get_token().await.unwrap();

        let req = client
            .get(CUBE_URL)
            .header(reqwest::header::AUTHORIZATION, format!("Token {}", &token));
        let res = req.send().await.unwrap();
        assert_eq!(res.status(), reqwest::StatusCode::OK);
    }

    #[rstest]
    #[tokio::test]
    async fn test_create_user(cube_url: CUBEApiUrl, client: reqwest::Client) {
        let mut generator = Generator::default();
        let username = generator.next().unwrap();
        let password = format!("{}1234", &username.chars().rev().collect::<String>());
        let email = format!("{}@example.org", &username);

        let account_creator = CUBEAuth {
            username: Username::new(username.clone()),
            password,
            url: cube_url,
            client,
        };

        if account_creator.get_token().await.is_ok() {
            panic!("Account already exists for username {}", username);
        }

        let created_account = account_creator.create_account(&email).await.unwrap();
        assert_eq!(*created_account.username.as_str(), username);
        assert_eq!(created_account.email, email);

        let _token = account_creator.get_token().await.unwrap();
    }
}
