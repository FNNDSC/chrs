//! Predecessors to [ChrisClient] for getting _ChRIS_ authorization
//! tokens or creating _ChRIS_ accounts.

use crate::types::{CubeUrl, ItemUrl, UserId, Username};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct AuthTokenResponse {
    // clippy doesn't know how serde works
    #[allow(dead_code)]
    token: String,
}

#[derive(Deserialize)]
pub struct UserCreatedResponse {
    pub url: ItemUrl,
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
/// [Account] is a builder for [ChrisClient].
pub struct Account<'a> {
    pub client: reqwest::Client,
    pub url: &'a CubeUrl,
    pub username: &'a Username,
    pub password: &'a str,
}

impl<'a> Account<'a> {
    pub fn new(url: &'a CubeUrl, username: &'a Username, password: &'a str) -> Self {
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
}
