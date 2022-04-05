//! chrs application configuration --- mainly just saving
//! the login token for CUBE, or possibly multiple CUBEs.
//!
//! The configuration file is saved to `~/.config/chrs/chrs.toml`.
//! The file contents are login tokens for CUBE stored in the
//! order they were added, i.e. the last element is the most
//! recently-added login token.

use crate::login::tokenstore::{Backend, Login, SavedCubeAuth};
use anyhow::{Context, Ok, Result};
use chris::types::{CUBEApiUrl, Username};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "org.chrisproject.chrs";
const APP_NAME: &str = "chrs";

/// Saved logins for chrs.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ChrsConfig {
    cubes: Vec<SavedCubeAuth>,
}

impl ChrsConfig {
    /// Get the [Login] corresponding to user-supplied address and username.
    /// If address is not given, the first set of credentials appearing in
    /// the configuration file is assumed to be for the default CUBE.
    pub fn get_login(
        &self,
        address: Option<&CUBEApiUrl>,
        username: Option<&Username>,
    ) -> Result<Option<Login>> {
        match self.get_cube(address, username) {
            None => Ok(None),
            Some(cube) => Ok(Some(cube.to_owned().into_login(SERVICE)?)),
        }
    }

    /// Get the credentials for a CUBE. If `address` is not specified, then
    /// return the most recently added login. A `username` for the `address`
    /// may be specified in cases where multiple logins for the same CUBE
    /// are saved.
    pub fn get_cube(
        &self,
        address: Option<&CUBEApiUrl>,
        username: Option<&Username>,
    ) -> Option<&SavedCubeAuth> {
        match address {
            None => self.cubes.last(),
            Some(a) => self.find_cube(a, username),
        }
    }

    fn find_cube(
        &self,
        address: &CUBEApiUrl,
        username: Option<&Username>,
    ) -> Option<&SavedCubeAuth> {
        for cube in &self.cubes {
            if address == &cube.address {
                if let Some(given_username) = username {
                    if given_username == &cube.username {
                        return Some(cube);
                    }
                } else {
                    return Some(cube);
                }
            }
        }
        None
    }

    /// Append the given [Login]. If there already exists in this [ChrsConfig]
    /// a token for the [Login]'s address and username, it is overwritten.
    pub fn add(&mut self, cube: Login, backend: Backend) -> Result<()> {
        self.remove(&cube.address, Some(&cube.username));
        self.cubes.push(cube.into_saved(backend, SERVICE)?);
        Ok(())
    }

    /// Remove saved login(s), Returns `true` if login was removed,
    /// or `false` if nothing was removed.
    pub fn remove(&mut self, address: &CUBEApiUrl, username: Option<&Username>) -> bool {
        fn keep(a: &SavedCubeAuth, address: &CUBEApiUrl, username: Option<&Username>) -> bool {
            if &a.address != address {
                return true;
            }
            if let Some(u) = username {
                if &a.username == u {
                    return false;
                }
                return true;
            }
            false
        }

        let original_len = self.cubes.len();
        self.cubes.retain(|l| keep(l, address, username));
        self.cubes.len() != original_len
    }

    /// Remove all saved logins. Returns `true` if any logins were removed.
    pub fn clear(&mut self) -> bool {
        let original_len = self.cubes.len();
        self.cubes.clear();
        original_len != 0
    }

    /// Load config from file.
    pub fn load() -> Result<Self> {
        let c: Self = confy::load(APP_NAME).context("Couldn't load config file")?;
        Ok(c)
    }

    /// Write config to file.
    pub fn store(&self) -> Result<()> {
        confy::store(APP_NAME, self).context("Couldn't write config file")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::tokenstore::StoredToken;
    use lazy_static::lazy_static;
    use std::str::FromStr;

    lazy_static! {
        static ref EXAMPLE_ADDRESS: CUBEApiUrl =
            CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap();
        static ref EXAMPLE_USERNAME: Username = Username::from_str("testing-chrs").unwrap();
        static ref EXAMPLE_CONFIG: ChrsConfig = ChrsConfig {
            cubes: vec![
                SavedCubeAuth {
                    address: CUBEApiUrl::from_str("https://a.example.com/api/v1/").unwrap(),
                    username: Username::from_str("aaaaa").unwrap(),
                    store: StoredToken::Text("token-a".to_string())
                },
                SavedCubeAuth {
                    address: CUBEApiUrl::from_str("https://b.example.com/api/v1/").unwrap(),
                    username: Username::from_str("b-first").unwrap(),
                    store: StoredToken::Text("token-b1".to_string())
                },
                SavedCubeAuth {
                    address: CUBEApiUrl::from_str("https://c.example.com/api/v1/").unwrap(),
                    username: Username::from_str("ccccc").unwrap(),
                    store: StoredToken::Keyring
                },
                SavedCubeAuth {
                    address: CUBEApiUrl::from_str("https://b.example.com/api/v1/").unwrap(),
                    username: Username::from_str("b-second").unwrap(),
                    store: StoredToken::Text("token-b2".to_string())
                },
            ]
        };
    }

    #[test]
    fn test_empty_config() -> Result<()> {
        let empty_config = ChrsConfig::default();
        assert!(empty_config.get_login(None, None)?.is_none());
        assert!(empty_config
            .get_login(Some(&EXAMPLE_ADDRESS), None)?
            .is_none());
        assert!(empty_config
            .get_login(None, Some(&EXAMPLE_USERNAME))?
            .is_none());
        assert!(empty_config
            .get_login(Some(&EXAMPLE_ADDRESS), Some(&EXAMPLE_USERNAME))?
            .is_none());
        Ok(())
    }

    #[test]
    fn test_get_default_cube() -> Result<()> {
        let expected = Login {
            address: CUBEApiUrl::from_str("https://b.example.com/api/v1/").unwrap(),
            username: Username::from_str("b-second").unwrap(),
            token: "token-b2".to_string(),
        };
        assert_eq!(Some(expected), EXAMPLE_CONFIG.get_login(None, None)?);
        Ok(())
    }

    #[test]
    fn test_get_cube_by_address() -> Result<()> {
        let addr = CUBEApiUrl::from_str("https://c.example.com/api/v1/").unwrap();

        assert_eq!(
            Some(&EXAMPLE_CONFIG.cubes[2]),
            EXAMPLE_CONFIG.find_cube(&addr, None)
        );
        assert_eq!(
            Some(&EXAMPLE_CONFIG.cubes[2]),
            EXAMPLE_CONFIG.find_cube(&addr, Some(&Username::from_str("ccccc").unwrap()))
        );
        assert_eq!(
            None,
            EXAMPLE_CONFIG.find_cube(&addr, Some(&Username::from_str("aaaaa").unwrap()))
        );
        Ok(())
    }

    #[test]
    fn test_same_cube_different_users() -> Result<()> {
        let address = CUBEApiUrl::from_str("https://b.example.com/api/v1/").unwrap();
        let expected1 = Login {
            address: address.clone(),
            username: Username::from_str("b-first").unwrap(),
            token: "token-b1".to_string(),
        };
        let expected2 = Login {
            address: address.clone(),
            username: Username::from_str("b-second").unwrap(),
            token: "token-b2".to_string(),
        };
        assert_eq!(
            Some(&expected1),
            EXAMPLE_CONFIG.get_login(Some(&address), None)?.as_ref()
        );
        assert_eq!(
            Some(&expected1),
            EXAMPLE_CONFIG
                .get_login(Some(&address), Some(&expected1.username))?
                .as_ref()
        );
        assert_eq!(
            Some(&expected2),
            EXAMPLE_CONFIG
                .get_login(Some(&address), Some(&expected2.username))?
                .as_ref()
        );
        Ok(())
    }

    #[test]
    fn test_add() -> Result<()> {
        let mut config = ChrsConfig::default();
        assert_eq!(0, config.cubes.len());
        config.add(
            Login {
                address: CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap(),
                username: Username::from_str("apple").unwrap(),
                token: "red-delicious".to_string(),
            },
            Backend::ClearText,
        )?;
        assert_eq!(1, config.cubes.len());
        assert_eq!(
            StoredToken::Text("red-delicious".to_string()),
            config
                .get_cube(
                    Some(&CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap()),
                    None
                )
                .unwrap()
                .store
        );

        config.add(
            Login {
                address: CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap(),
                username: Username::from_str("apple").unwrap(),
                token: "golden-delicious".to_string(),
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            1,
            config.cubes.len(),
            "length is not the same after adding a Login with same address and username"
        );
        assert_eq!(
            StoredToken::Text("golden-delicious".to_string()),
            config
                .get_cube(
                    Some(&CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap()),
                    None
                )
                .unwrap()
                .store
        );

        config.add(
            Login {
                address: CUBEApiUrl::from_str("https://example.com/api/v1/").unwrap(),
                username: Username::from_str("pear").unwrap(),
                token: "yapearisachinesepear".to_string(),
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            2,
            config.cubes.len(),
            "length did not increase after adding a login with a different username."
        );

        config.add(
            Login {
                address: CUBEApiUrl::from_str("https://another.example.com/api/v1/").unwrap(),
                username: Username::from_str("pear").unwrap(),
                token: "yapearisachinesepear".to_string(),
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            3,
            config.cubes.len(),
            "length did not increase after adding a login with a different address."
        );
        Ok(())
    }

    #[test]
    fn test_remove() -> Result<()> {
        let mut config = ChrsConfig::default();
        config.add(
            Login {
                address: CUBEApiUrl::from_str("https://one.example.com/api/v1/").unwrap(),
                username: Username::from_str("apple").unwrap(),
                token: "red-delicious".to_string(),
            },
            Backend::ClearText,
        )?;
        config.add(
            Login {
                address: CUBEApiUrl::from_str("https://two.example.com/api/v1/").unwrap(),
                username: Username::from_str("pear").unwrap(),
                token: "yapearisachinesepear".to_string(),
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            StoredToken::Text("red-delicious".to_string()),
            config
                .get_cube(
                    Some(&CUBEApiUrl::from_str("https://one.example.com/api/v1/").unwrap()),
                    None
                )
                .unwrap()
                .store
        );
        assert_eq!(
            StoredToken::Text("yapearisachinesepear".to_string()),
            config
                .get_cube(
                    Some(&CUBEApiUrl::from_str("https://two.example.com/api/v1/").unwrap()),
                    None
                )
                .unwrap()
                .store
        );

        let addr1 = CUBEApiUrl::from_str("https://one.example.com/api/v1/").unwrap();
        assert!(config.remove(&addr1, None));
        assert!(!config.remove(&addr1, None), "login already removed");
        assert!(config.get_cube(Some(&addr1), None).is_none());

        let addr2 = CUBEApiUrl::from_str("https://two.example.com/api/v1/").unwrap();
        assert_eq!(
            StoredToken::Text("yapearisachinesepear".to_string()),
            config.get_cube(Some(&addr2), None).unwrap().store
        );
        assert!(
            !config.remove(&addr2, Some(&Username::from_str("apple").unwrap())),
            "username should not be found"
        );
        assert!(config.remove(&addr2, Some(&Username::from_str("pear").unwrap())));
        assert!(config.get_cube(Some(&addr2), None).is_none());

        Ok(())
    }
}
