//! chrs application configuration --- mainly just saving
//! the login token for CUBE, or possibly multiple CUBEs.

use crate::login::tokenstore::{Backend, Login, SavedCubeAuth};
use anyhow::{bail, Context, Ok, Result};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "org.chrisproject.chrs";
const APP_NAME: &str = "chrs";

/// Saved logins for chrs.
#[derive(Serialize, Deserialize)]
pub struct ChrsConfig {
    cubes: Vec<SavedCubeAuth>,
}

impl Default for ChrsConfig {
    fn default() -> Self {
        ChrsConfig { cubes: vec![] }
    }
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

    /// Get the credentials for a CUBE. If `address` is not specified, then
    /// return the most recently added login. A `username` for the `address`
    /// may be specified in cases where multiple logins for the same CUBE
    /// are saved.
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

    /// Append the given [Login]. If there already exists in this [ChrsConfig]
    /// a token for the [Login]'s address and username, it is overwritten.
    pub fn add(&mut self, cube: Login, backend: Backend) -> Result<()> {
        self.remove(&cube.address, Some(&cube.username));
        self.cubes.push(cube.to_saved(backend, SERVICE)?);
        Ok(())
    }

    /// Remove a saved login, if found. Returns `true` if login was removed,
    /// or `false` if nothing was removed.
    pub fn remove(&mut self, address: &str, username: Option<&str>) -> bool {
        if let Some(i) = self.index_of(address, username) {
            self.cubes.remove(i);
            return true;
        }
        false
    }

    fn index_of(&self, address: &str, username: Option<&str>) -> Option<usize> {
        for (i, e) in self.cubes.iter().enumerate() {
            if e.address == address {
                if let Some(u) = username {
                    if u == e.username {
                        return Some(i);
                    }
                } else {
                    return Some(i);
                }
            }
        }
        return None;
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
        let empty_config = ChrsConfig::default();
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

    #[test]
    fn test_add() -> Result<()> {
        let mut config = ChrsConfig::default();
        assert_eq!(0, config.cubes.len());
        config.add(
            Login {
                address: String::from("https://example.com/api/v1/"),
                username: String::from("apple"),
                token: String::from("red-delicious"),
            },
            Backend::ClearText,
        );
        assert_eq!(1, config.cubes.len());
        assert_eq!(
            StoredToken::Text(String::from("red-delicious")),
            config
                .get_cube(Some("https://example.com/api/v1/"), None)
                .unwrap()
                .store
        );

        config.add(
            Login {
                address: String::from("https://example.com/api/v1/"),
                username: String::from("apple"),
                token: String::from("golden-delicious"),
            },
            Backend::ClearText,
        );
        assert_eq!(
            1,
            config.cubes.len(),
            "length is not the same after adding a Login with same address and username"
        );
        assert_eq!(
            StoredToken::Text(String::from("golden-delicious")),
            config
                .get_cube(Some("https://example.com/api/v1/"), None)
                .unwrap()
                .store
        );

        config.add(
            Login {
                address: String::from("https://example.com/api/v1/"),
                username: String::from("pear"),
                token: String::from("yapearisachinesepear"),
            },
            Backend::ClearText,
        );
        assert_eq!(
            2,
            config.cubes.len(),
            "length did not increase after adding a login with a different username."
        );

        config.add(
            Login {
                address: String::from("https://another.example.com/api/v1/"),
                username: String::from("pear"),
                token: String::from("yapearisachinesepear"),
            },
            Backend::ClearText,
        );
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
                address: String::from("https://one.example.com/api/v1/"),
                username: String::from("apple"),
                token: String::from("red-delicious"),
            },
            Backend::ClearText,
        );
        config.add(
            Login {
                address: String::from("https://two.example.com/api/v1/"),
                username: String::from("pear"),
                token: String::from("yapearisachinesepear"),
            },
            Backend::ClearText,
        );
        assert_eq!(
            StoredToken::Text(String::from("red-delicious")),
            config
                .get_cube(Some("https://one.example.com/api/v1/"), None)
                .unwrap()
                .store
        );
        println!("{:?}", config.cubes);
        assert_eq!(
            StoredToken::Text(String::from("yapearisachinesepear")),
            config
                .get_cube(Some("https://two.example.com/api/v1/"), None)
                .unwrap()
                .store
        );
        assert!(config.remove("https://one.example.com/api/v1/", None));
        assert!(
            !config.remove("https://one.example.com/api/v1/", None),
            "login already removed"
        );
        assert!(config
            .get_cube(Some("https://one.example.com/api/v1/"), None)
            .is_none());
        assert_eq!(
            StoredToken::Text(String::from("yapearisachinesepear")),
            config
                .get_cube(Some("https://two.example.com/api/v1/"), None)
                .unwrap()
                .store
        );
        assert!(
            !config.remove("https://two.example.com/api/v1/", Some("apple")),
            "username should not be found"
        );
        assert!(config.remove("https://two.example.com/api/v1/", Some("pear")));
        assert!(config
            .get_cube(Some("https://two.example.com/api/v1/"), None)
            .is_none());

        Ok(())
    }
}
