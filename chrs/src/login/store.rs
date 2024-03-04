//! Abstraction over token storage using keyring or in plaintext configuration file.
//! When saved to keyring, the token is identified by a string in the form
//! "<CUBEUsername>@<CUBEAddress>"

use chris::types::{CubeUrl, Username};
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

/// Supported mechanisms for storing secrets.
pub enum Backend {
    ClearText,
    Keyring,
}

/// A secret which may be securely stored.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
#[serde(tag = "store", content = "value")]
pub enum StoredToken {
    Text(String),
    Keyring,
    None,
}

/// A [SavedCubeAuth] is a precursor to [Login] which is what is stored
/// in the application's configuration file. The token might be stored
/// in the same file as plaintext, or it might be stored by a keyring.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct SavedCubeAuth {
    pub cube: CubeUrl,
    pub username: Username,
    pub store: StoredToken,
}

impl SavedCubeAuth {
    /// Convert this [SavedCubeAuth] to a [Login]. In the case where the
    /// token is stored by a keyring, fetch it from the keyring.
    pub fn into_login(self, service: &str) -> Result<Login> {
        let token = match &self.store {
            StoredToken::Keyring => {
                let entry = keyring::Entry::new(service, &self.to_keyring_username());
                let token = entry.get_password()?;
                Ok::<_, keyring::Error>(Some(token))
            }
            StoredToken::Text(token) => Ok(Some(token.to_owned())),
            StoredToken::None => Ok(None),
        }?;
        Ok(Login {
            cube: self.cube,
            username: self.username,
            token,
        })
    }

    fn to_keyring_username(&self) -> String {
        format!("{}@{}", self.username.as_str(), self.cube.as_str())
    }
}

/// A [Login] is the data required to authenticate with CUBE.
/// If username is empty, then the client is anonymous.
#[derive(Eq, PartialEq, Debug)]
pub struct Login {
    pub cube: CubeUrl,
    pub username: Username,
    pub token: Option<String>,
}

impl Login {
    /// Convert to [SavedCubeAuth]. If specified to use keyring backend,
    /// token is saved to the keyring.
    pub fn into_saved(self, backend: Backend, service: &str) -> Result<SavedCubeAuth> {
        let token: StoredToken = if let Some(token) = &self.token {
            match backend {
                Backend::ClearText => StoredToken::Text(token.to_string()),
                Backend::Keyring => {
                    let entry = keyring::Entry::new(service, &self.to_keyring_username());
                    entry.set_password(&token)?; // TODO suggest trying again with --no-keyring
                    StoredToken::Keyring
                }
            }
        } else {
            StoredToken::None
        };
        let saved = SavedCubeAuth {
            username: self.username,
            cube: self.cube,
            store: token,
        };
        Ok(saved)
    }

    fn to_keyring_username(&self) -> String {
        format!("{}@{}", self.username.as_str(), self.cube.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::str::FromStr;

    const TEST_SERVICE: &str = "org.chrisproject.chrs.test";

    #[rstest]
    fn test_into_login_from_text(cube_url: CubeUrl, username: Username) -> Result<()> {
        let token = "my-secret-token";
        let (expected, actual) = login_helper(
            &cube_url,
            &username,
            StoredToken::Text(token.to_string()),
            &token,
        )?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[fixture]
    fn username() -> Username {
        Username::from_static("chrs-testuser")
    }

    #[fixture]
    fn cube_url() -> CubeUrl {
        CubeUrl::from_static("https://example.org/api/v1/")
    }

    fn login_helper(
        cube_url: &CubeUrl,
        username: &Username,
        stored_token: StoredToken,
        actual_token: &str,
    ) -> Result<(Login, Login)> {
        let cube = SavedCubeAuth {
            cube: cube_url.clone(),
            username: username.clone(),
            store: stored_token,
        };
        let login = Login {
            cube: cube_url.clone(),
            username: username.clone(),
            token: Some(actual_token.to_string()),
        };
        Ok((login, cube.into_login(TEST_SERVICE)?))
    }

    #[rstest]
    fn test_into_login_from_keyring(username: Username, cube_url: CubeUrl) -> Result<()> {
        let token = "my-secret-secure-token";
        let keyring_username = format!("{}@{}", username.as_str(), cube_url.as_str());
        let entry = keyring::Entry::new(TEST_SERVICE, &*keyring_username);
        entry.set_password(&token)?;

        let (expected, actual) = login_helper(&cube_url, &username, StoredToken::Keyring, token)?;
        entry.delete_password()?;

        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_into_saved_with_keyring() -> Result<()> {
        let token = "my-secret-secure-token-again";
        let address = CubeUrl::from_static("https://another.example.com/api/v1/");
        let username = Username::from_str("testing-chrs").unwrap();
        let login = Login {
            cube: address.to_owned(),
            username: username.to_owned(),
            token: Some(token.to_string()),
        };
        login.into_saved(Backend::Keyring, TEST_SERVICE)?;

        let keyring_username = format!("{}@{}", username.as_str(), address.as_str());
        let entry = keyring::Entry::new(TEST_SERVICE, &*keyring_username);
        assert_eq!(token, entry.get_password()?);
        entry.delete_password()?;
        Ok(())
    }
}
