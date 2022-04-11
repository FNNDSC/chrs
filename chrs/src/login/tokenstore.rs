//! Abstraction over token storage using keyring or in plaintext configuration file.
//! When saved to keyring, the token is identified by a string in the form
//! "<CUBEUsername>@<CUBEAddress>"

use anyhow::{Context, Ok, Result};
use chris::common_types::{CUBEApiUrl, Username};
use chris::ChrisClient;
use console::style;
use serde::{Deserialize, Serialize};

/// Supported mechanisms for storing secrets.
pub enum Backend {
    ClearText,
    Keyring,
}

/// A secret which may be securely stored.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(tag = "store", content = "value")]
pub enum StoredToken {
    Text(String),
    Keyring,
}

/// A [SavedCubeAuth] is a precursor to [Login] which is what is stored
/// in the application's configuration file. The token might be stored
/// in the same file as plaintext, or it might be stored by a keyring.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct SavedCubeAuth {
    pub address: CUBEApiUrl,
    pub username: Username,
    pub store: StoredToken,
}

impl SavedCubeAuth {
    /// Convert this [SavedCubeAuth] to a [Login]. In the case where the
    /// token is stored by a keyring, fetch it from the keyring.
    pub fn into_login(self, service: &str) -> Result<Login> {
        let token = match &self.store {
            StoredToken::Text(token) => Ok(token.to_owned()),
            StoredToken::Keyring => {
                let entry = keyring::Entry::new(service, &*self.to_keyring_username());
                let token = entry.get_password().with_context(|| {
                    format!(
                        "Could not get login token from keyring \
                        with service=\"{}\" and username=\"{}\"",
                        &service,
                        self.username.as_str()
                    )
                })?;
                Ok(token)
            }
        }?;
        Ok(Login {
            address: self.address,
            username: self.username,
            token,
        })
    }

    fn to_keyring_username(&self) -> String {
        format!("{}@{}", self.username.as_str(), self.address.as_str())
    }
}

/// A [Login] is the data required to authenticate with CUBE.
#[derive(PartialEq, Debug)]
pub struct Login {
    pub address: CUBEApiUrl,
    pub username: Username,
    pub token: String,
}

impl Login {
    /// Convert to [SavedCubeAuth]. If specified to use keyring backend,
    /// token is saved to the keyring.
    pub fn into_saved(self, backend: Backend, service: &str) -> Result<SavedCubeAuth> {
        let token: StoredToken = match backend {
            Backend::ClearText => StoredToken::Text(self.token),
            Backend::Keyring => {
                let entry = keyring::Entry::new(service, &*self.to_keyring_username());
                entry.set_password(&self.token)?;
                StoredToken::Keyring
            }
        };
        let saved = SavedCubeAuth {
            username: self.username,
            address: self.address,
            store: token,
        };
        Ok(saved)
    }

    pub async fn into_client(self) -> Result<ChrisClient> {
        let client = ChrisClient::new(self.address, self.username, self.token)
            .await
            .with_context(|| {
                format!(
                    "Could not login. \
            Your token may have expired, please run {}",
                    style("chrs logout").bold()
                )
            })?;
        Ok(client)
    }

    fn to_keyring_username(&self) -> String {
        format!("{}@{}", self.username.as_str(), self.address.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::str::FromStr;

    lazy_static! {
        static ref EXAMPLE_ADDRESS: CUBEApiUrl =
            CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap();
        static ref EXAMPLE_USERNAME: Username = Username::from_str("testing-chrs").unwrap();
    }

    const TEST_SERVICE: &str = "org.chrisproject.chrs.test";

    #[test]
    fn test_into_login_from_text() -> Result<()> {
        let token = "my-secret-token";
        let (expected, actual) = login_helper(StoredToken::Text(token.to_string()), &token)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_into_login_from_keyring() -> Result<()> {
        let token = "my-secret-secure-token";
        let keyring_username =
            format!("{}@{}", EXAMPLE_USERNAME.as_str(), EXAMPLE_ADDRESS.as_str());
        let entry = keyring::Entry::new(TEST_SERVICE, &*keyring_username);
        entry.set_password(&token)?;

        let (expected, actual) = login_helper(StoredToken::Keyring, token)?;
        entry.delete_password()?;

        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_into_saved_with_keyring() -> Result<()> {
        let token = "my-secret-secure-token-again";
        let address = CUBEApiUrl::from_str("https://another.example.com/api/v1/").unwrap();
        let username = Username::from_str("testing-chrs").unwrap();
        let login = Login {
            address: address.to_owned(),
            username: username.to_owned(),
            token: token.to_string(),
        };
        login.into_saved(Backend::Keyring, TEST_SERVICE)?;

        let keyring_username = format!("{}@{}", username.as_str(), address.as_str());
        let entry = keyring::Entry::new(TEST_SERVICE, &*keyring_username);
        assert_eq!(
            token,
            entry.get_password().with_context(|| format!(
                "No secret in keyring for keyring_username={}",
                keyring_username
            ))?
        );
        entry.delete_password()?;
        Ok(())
    }

    fn login_helper(stored_token: StoredToken, actual_token: &str) -> Result<(Login, Login)> {
        let cube = SavedCubeAuth {
            address: EXAMPLE_ADDRESS.clone(),
            username: EXAMPLE_USERNAME.clone(),
            store: stored_token,
        };
        let login = Login {
            address: EXAMPLE_ADDRESS.clone(),
            username: EXAMPLE_USERNAME.clone(),
            token: actual_token.to_string(),
        };
        Ok((login, cube.into_login(TEST_SERVICE)?))
    }
}
