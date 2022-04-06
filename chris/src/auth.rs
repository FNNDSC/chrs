use crate::common_types::{CUBEApiUrl, Username};
use serde::{Deserialize, Serialize};
use crate::api::{UserId, UserUrl};

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

pub struct CUBEAuth<'a> {
    pub client: &'a reqwest::Client,
    pub url: &'a CUBEApiUrl,
    pub username: &'a Username,
    pub password: &'a str,
}

impl CUBEAuth<'_> {
    pub async fn get_token(&self) -> Result<String, reqwest::Error> {
        let auth_url = format!("{}auth-token/", &self.url);
        let req = self
            .client
            .post(auth_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&Credentials {
                username: self.username,
                password: self.password,
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
                username: self.username,
                password: self.password,
                email,
            });
        let res = req.send().await?;
        res.error_for_status_ref()?;
        let created_user: UserCreatedResponse = res.json().await?;
        Ok(created_user)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use names::Generator;
    use std::str::FromStr;

    const CUBE_URL: &str = "http://localhost:8000/api/v1/";

    #[tokio::test]
    async fn test_get_token() -> Result<(), Box<dyn std::error::Error>> {
        let account = CUBEAuth {
            username: &Username::from_str("chris")?,
            password: "chris1234",
            url: &CUBE_API_URL,
            client: &CLIENT,
        };

        let token = account.get_token().await?;

        let req = CLIENT
            .get(&CUBE_API_URL.to_string())
            .header(reqwest::header::AUTHORIZATION, format!("Token {}", &token));
        let res = req.send().await?;
        assert_eq!(res.status(), reqwest::StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_user() -> Result<(), Box<dyn std::error::Error>> {
        let mut generator = Generator::default();
        let username = generator.next().unwrap();
        let email = format!("{}@example.org", &username);

        let account_creator = CUBEAuth {
            username: &Username::from_str(username.as_str())?,
            password: &*format!("{}1234", username.chars().rev().collect::<String>()),
            url: &CUBE_API_URL,
            client: &CLIENT,
        };

        if account_creator.get_token().await.is_ok() {
            panic!("Account already exists for username {}", username);
        }

        let created_account = account_creator.create_account(&email).await?;
        assert_eq!(*created_account.username.as_str(), username);
        assert_eq!(created_account.email, email);

        let _token = account_creator.get_token().await?;
        Ok(())
    }

    lazy_static! {
        static ref CLIENT: reqwest::Client = reqwest::Client::new();
        static ref CUBE_API_URL: CUBEApiUrl = CUBEApiUrl::new(CUBE_URL).unwrap();
    }
}
