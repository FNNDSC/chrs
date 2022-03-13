//! chrs application configuration --- mainly just saving
//! the login token for CUBE, or possibly multiple CUBEs.

use crate::login::{Login, SavedCubeAuth};
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "org.chrisproject.chrs";

/// Saved logins for chrs.
#[derive(Serialize, Deserialize)]
pub struct ChrsConfig {
    cubes: Vec<SavedCubeAuth>,
}

impl ChrsConfig {
    /// Get the [Login] corresponding to user-supplied address and username.
    /// If address is not given, the first set of credentials appearing in
    /// the configuration file is assumed to be for the default CUBE.
    pub fn get_login(
        &self,
        address: Option<&str>,
        username: Option<&str>,
    ) -> Result<Option<Login>> {
        match self.get_cube(address, username) {
            None => Ok(None),
            Some(cube) => Ok(Some(cube.to_login(SERVICE)?)),
        }
    }

    pub fn get_cube(
        &self,
        address: Option<&str>,
        username: Option<&str>,
    ) -> Option<&SavedCubeAuth> {
        match address {
            None => self.cubes.first(),
            Some(a) => self.find_cube(a, username),
        }
    }

    fn find_cube(&self, address: &str, username: Option<&str>) -> Option<&SavedCubeAuth> {
        for cube in &self.cubes {
            if address == cube.address {
                if let Some(given_username) = username {
                    if given_username == cube.username {
                        return Some(cube);
                    }
                } else {
                    return Some(cube);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::StoredToken;
    use lazy_static::lazy_static;

    const EXAMPLE_ADDRESS: &str = "https://example.com/api/v1/";
    const EXAMPLE_USERNAME: &str = "testing-chrs";

    lazy_static! {
        static ref EXAMPLE_CONFIG: ChrsConfig = ChrsConfig {
            cubes: vec![
                SavedCubeAuth {
                    address: "https://a.example.com/api/v1/".to_string(),
                    username: "aaaaa".to_string(),
                    store: StoredToken::Text("token-a".to_string())
                },
                SavedCubeAuth {
                    address: "https://b.example.com/api/v1/".to_string(),
                    username: "b-first".to_string(),
                    store: StoredToken::Text("token-b1".to_string())
                },
                SavedCubeAuth {
                    address: "https://c.example.com/api/v1/".to_string(),
                    username: "ccccc".to_string(),
                    store: StoredToken::Keyring
                },
                SavedCubeAuth {
                    address: "https://b.example.com/api/v1/".to_string(),
                    username: "b-second".to_string(),
                    store: StoredToken::Text("token-b2".to_string())
                },
            ]
        };
    }

    #[test]
    fn test_empty_config() -> Result<()> {
        let empty_config = ChrsConfig { cubes: vec![] };
        assert!(empty_config.get_login(None, None)?.is_none());
        assert!(empty_config
            .get_login(Some(EXAMPLE_ADDRESS), None)?
            .is_none());
        assert!(empty_config
            .get_login(None, Some(EXAMPLE_USERNAME))?
            .is_none());
        assert!(empty_config
            .get_login(Some(EXAMPLE_ADDRESS), Some(EXAMPLE_USERNAME))?
            .is_none());
        Ok(())
    }

    #[test]
    fn test_get_default_cube() -> Result<()> {
        let expected = Login {
            address: "https://a.example.com/api/v1/".to_string(),
            username: "aaaaa".to_string(),
            token: "token-a".to_string(),
        };
        assert_eq!(Some(expected), EXAMPLE_CONFIG.get_login(None, None)?);
        Ok(())
    }

    #[test]
    fn test_get_cube_by_address() -> Result<()> {
        assert_eq!(
            Some(&EXAMPLE_CONFIG.cubes[2]),
            EXAMPLE_CONFIG.find_cube("https://c.example.com/api/v1/", None)
        );
        assert_eq!(
            Some(&EXAMPLE_CONFIG.cubes[2]),
            EXAMPLE_CONFIG.find_cube("https://c.example.com/api/v1/", Some("ccccc"))
        );
        assert_eq!(
            None,
            EXAMPLE_CONFIG.find_cube("https://c.example.com/api/v1/", Some("aaaaa"))
        );
        Ok(())
    }

    #[test]
    fn test_same_cube_different_users() -> Result<()> {
        let address = "https://b.example.com/api/v1/";
        let expected1 = Login {
            address: address.to_string(),
            username: "b-first".to_string(),
            token: "token-b1".to_string(),
        };
        let expected2 = Login {
            address: address.to_string(),
            username: "b-second".to_string(),
            token: "token-b2".to_string(),
        };
        assert_eq!(
            Some(&expected1),
            EXAMPLE_CONFIG.get_login(Some(&address), None)?.as_ref()
        );
        assert_eq!(
            Some(&expected1),
            EXAMPLE_CONFIG
                .get_login(Some(&address), Some("b-first"))?
                .as_ref()
        );
        assert_eq!(
            Some(&expected2),
            EXAMPLE_CONFIG
                .get_login(Some(&address), Some("b-second"))?
                .as_ref()
        );
        Ok(())
    }
}
