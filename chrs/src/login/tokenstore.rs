//! Abstraction over token storage using keyring or in plaintext configuration file.
//! When saved to keyring, the token is identified by a string in the form
//! "<CUBEUsername>@<CUBEAddress>"

use anyhow::{Context, Ok, Result};
use serde::{Deserialize, Serialize};

/// A [SavedCubeAuth] is a precursor to [Login] which is what is stored
/// in the application's configuration file. The token might be stored
/// in the same file as plaintext, or it might be stored by a keyring.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct SavedCubeAuth {
    pub address: String,
    pub username: String,
    pub store: StoredToken,
}

impl SavedCubeAuth {
    /// Convert this [SavedCubeAuth] to a [Login]. In the case where the
    /// token is stored by a keyring, fetch it from the keyring.
    pub fn to_login(&self, service: &str) -> Result<Login> {
        let token = match &self.store {
            StoredToken::Text(token) => Ok(token.to_owned()),
            StoredToken::Keyring => {
                let entry = keyring::Entry::new(service, &*self.to_keyring_username());
                let token = entry.get_password().with_context(|| {
                    format!(
                        "Could not get login token from keyring \
                        with service=\"{}\" and username=\"{}\"",
                        &service, &self.username
                    )
                })?;
                Ok(token)
            }
        }?;
        Ok(Login {
            address: (&self.address).to_owned(),
            username: (&self.username).to_owned(),
            token,
        })
    }

    fn to_keyring_username(&self) -> String {
        format!("{}@{}", &self.username, &self.address)
    }
}

/// A [Login] is the data required to authenticate with CUBE.
#[derive(PartialEq, Debug)]
pub struct Login {
    pub address: String,
    pub username: String,
    pub token: String,
}

impl Login {
    /// Convert to [SavedCubeAuth]. If specified to use keyring backend,
    /// token is saved to the keyring.
    pub fn to_saved(self, backend: Backend, service: &str) -> Result<SavedCubeAuth> {
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

    fn to_keyring_username(&self) -> String {
        format!("{}@{}", &self.username, &self.address)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "store", content = "value")]
pub enum StoredToken {
    Text(String),
    Keyring,
}

pub enum Backend {
    ClearText,
    Keyring,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_ADDRESS: &str = "https://example.com/api/v1/";
    const EXAMPLE_USERNAME: &str = "testing-chrs";

    const TEST_SERVICE: &str = "org.chrisproject.chrs.test";

    #[test]
    fn test_to_login_from_text() -> Result<()> {
        let token = "my-secret-token";
        let (expected, actual) = login_helper(StoredToken::Text(token.to_string()), &token)?;
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_to_login_from_keyring() -> Result<()> {
        let token = "my-secret-secure-token";
        let keyring_username = format!("{}@{}", EXAMPLE_USERNAME, EXAMPLE_ADDRESS);
        let entry = keyring::Entry::new(TEST_SERVICE, &*keyring_username);
        entry.set_password(&token)?;

        let (expected, actual) = login_helper(StoredToken::Keyring, token)?;
        entry.delete_password()?;

        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_to_saved_with_keyring() -> Result<()> {
        let token = "my-secret-secure-token";
        let login = Login {
            address: EXAMPLE_ADDRESS.to_string(),
            username: EXAMPLE_USERNAME.to_string(),
            token: token.to_string(),
        };
        login.to_saved(Backend::Keyring, TEST_SERVICE)?;

        let keyring_username = format!("{}@{}", EXAMPLE_USERNAME, EXAMPLE_ADDRESS);
        let entry = keyring::Entry::new(TEST_SERVICE, &*keyring_username);
        assert_eq!(token, entry.get_password()?);
        entry.delete_password()?;
        Ok(())
    }

    fn login_helper(stored_token: StoredToken, actual_token: &str) -> Result<(Login, Login)> {
        let cube = SavedCubeAuth {
            address: EXAMPLE_ADDRESS.to_string(),
            username: EXAMPLE_USERNAME.to_string(),
            store: stored_token,
        };
        let login = Login {
            address: EXAMPLE_ADDRESS.to_string(),
            username: EXAMPLE_USERNAME.to_string(),
            token: actual_token.to_string(),
        };
        Ok((login, cube.to_login(TEST_SERVICE)?))
    }
}
