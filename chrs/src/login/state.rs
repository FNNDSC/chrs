use crate::login::store::{Backend, CubeState, SavedCubeState};
use chris::types::{CubeUrl, PluginInstanceId, Username};
use color_eyre::eyre::{Result, WrapErr};
use color_eyre::owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "org.chrisproject.chrs";
const APP_NAME: &str = "chrs";

/// The application state is a list of user sessions represented by [SavedCubeState].
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ChrsSessions {
    pub sessions: Vec<SavedCubeState>,
}

impl ChrsSessions {
    /// Get the [CubeState] corresponding to user-supplied address, username,
    /// and any additional arguments.
    /// If address is not given, the first set of credentials appearing in
    /// the configuration file is returned.
    pub fn get_login(
        &self,
        cube: Option<&CubeUrl>,
        username: Option<&Username>,
    ) -> Result<Option<CubeState>> {
        match self.get_cube(cube, username) {
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
        cube: Option<&CubeUrl>,
        username: Option<&Username>,
    ) -> Option<&SavedCubeState> {
        match cube {
            None => self.sessions.last(),
            Some(cube_url) => self.find_cube(cube_url, username),
        }
    }

    fn find_cube(
        &self,
        cube_url: &CubeUrl,
        username: Option<&Username>,
    ) -> Option<&SavedCubeState> {
        for cube in &self.sessions {
            if cube_url == &cube.cube {
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

    /// Append the given [CubeState]. If there already exists in this [ChrsSessions]
    /// a token for the [CubeState]'s address and username, it is overwritten.
    pub fn add(&mut self, session: CubeState, backend: Backend) -> Result<()> {
        self.remove(&session.cube, Some(&session.username));
        self.sessions.push(session.into_saved(backend, SERVICE)?);
        Ok(())
    }

    /// Remove saved login(s), Returns `true` if login was removed,
    /// or `false` if nothing was removed.
    pub fn remove(&mut self, cube_url: &CubeUrl, username: Option<&Username>) -> bool {
        fn keep(a: &SavedCubeState, cube_url: &CubeUrl, username: Option<&Username>) -> bool {
            if &a.cube != cube_url {
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

        let original_len = self.sessions.len();
        self.sessions.retain(|l| keep(l, cube_url, username));
        self.sessions.len() != original_len
    }

    /// Set the preferred login account by swapping its position to the end.
    pub fn set_last(&mut self, b: usize) {
        let a = self.sessions.len() - 1;
        self.sessions.swap(a, b)
    }

    /// Remove all saved logins. Returns `true` if any logins were removed.
    pub fn clear(&mut self) -> bool {
        let original_len = self.sessions.len();
        self.sessions.clear();
        original_len != 0
    }

    /// Load config from file.
    pub fn load() -> Result<Self> {
        let c: Self = confy::load(APP_NAME, None)
            .wrap_err_with(|| format!("Could not load config file. If chrs was upgraded from an old version, please run `{}`", "rm -rf ~/.config/chrs".bold()))?;
        Ok(c)
    }

    /// Write config to file.
    pub fn save(&self) -> Result<()> {
        confy::store(APP_NAME, None, self).wrap_err("Couldn't write config file")
    }

    /// Set the plugin instance of a session.
    pub fn set_plugin_instance(
        &mut self,
        cube_url: &CubeUrl,
        username: &Username,
        plinst: PluginInstanceId,
    ) -> bool {
        for session in &mut self.sessions {
            if &session.cube == cube_url && &session.username == username {
                session.current_plugin_instance_id = Some(plinst);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::store::StoredToken;
    use chris::types::PluginInstanceId;
    use rstest::*;
    use std::str::FromStr;

    #[fixture]
    fn sessions() -> Vec<SavedCubeState> {
        vec![
            SavedCubeState {
                cube: CubeUrl::from_static("https://a.example.com/api/v1/"),
                username: Username::from_str("aaaaa").unwrap(),
                store: StoredToken::Text("token-a".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            SavedCubeState {
                cube: CubeUrl::from_static("https://b.example.com/api/v1/"),
                username: Username::from_str("b-first").unwrap(),
                store: StoredToken::Text("token-b1".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            SavedCubeState {
                cube: CubeUrl::from_static("https://c.example.com/api/v1/"),
                username: Username::from_str("ccccc").unwrap(),
                store: StoredToken::Keyring,
                current_plugin_instance_id: None,
                ui: None,
            },
            SavedCubeState {
                cube: CubeUrl::from_static("https://b.example.com/api/v1/"),
                username: Username::from_str("b-second").unwrap(),
                store: StoredToken::Text("token-b2".to_string()),
                current_plugin_instance_id: Some(PluginInstanceId(43)),
                ui: None,
            },
        ]
    }

    #[fixture]
    fn chrs_sessions(sessions: Vec<SavedCubeState>) -> ChrsSessions {
        ChrsSessions { sessions }
    }

    #[fixture]
    fn example_cube_url() -> CubeUrl {
        CubeUrl::from_static("https://example.example.org/api/v1/")
    }

    #[fixture]
    fn example_username() -> Username {
        Username::from_static("chrs-example-username")
    }

    #[rstest]
    fn test_empty_config(example_cube_url: CubeUrl, example_username: Username) -> Result<()> {
        let empty_config = ChrsSessions::default();
        assert!(empty_config.get_login(None, None)?.is_none());
        assert!(empty_config
            .get_login(Some(&example_cube_url), None)?
            .is_none());
        assert!(empty_config
            .get_login(None, Some(&example_username))?
            .is_none());
        assert!(empty_config
            .get_login(Some(&example_cube_url), Some(&example_username),)?
            .is_none());
        Ok(())
    }

    #[rstest]
    fn test_get_default_cube(chrs_sessions: ChrsSessions) -> Result<()> {
        let expected = CubeState {
            cube: CubeUrl::from_static("https://b.example.com/api/v1/"),
            username: Username::from_static("b-second"),
            token: Some("token-b2".to_string()),
            current_plugin_instance_id: Some(PluginInstanceId(43)),
            ui: None,
        };
        assert_eq!(Some(expected), chrs_sessions.get_login(None, None)?);
        Ok(())
    }

    #[rstest]
    fn test_get_cube_by_address(chrs_sessions: ChrsSessions) -> Result<()> {
        let cube_url = CubeUrl::from_static("https://c.example.com/api/v1/");
        assert_eq!(
            Some(&chrs_sessions.sessions[2]),
            chrs_sessions.find_cube(&cube_url, None)
        );
        assert_eq!(
            Some(&chrs_sessions.sessions[2]),
            chrs_sessions.find_cube(&cube_url, Some(&Username::from_str("ccccc").unwrap()))
        );
        assert_eq!(
            None,
            chrs_sessions.find_cube(&cube_url, Some(&Username::from_str("aaaaa").unwrap()))
        );
        Ok(())
    }

    #[rstest]
    fn test_same_cube_different_users(chrs_sessions: ChrsSessions) -> Result<()> {
        let cube_url = CubeUrl::from_static("https://b.example.com/api/v1/");
        let expected1 = CubeState {
            cube: cube_url.clone(),
            username: Username::from_static("b-first"),
            token: Some("token-b1".to_string()),
            current_plugin_instance_id: None,
            ui: None,
        };
        let expected2 = CubeState {
            cube: cube_url.clone(),
            username: Username::from_static("b-second"),
            token: Some("token-b2".to_string()),
            current_plugin_instance_id: Some(PluginInstanceId(43)),
            ui: None,
        };
        assert_eq!(
            Some(&expected1),
            chrs_sessions.get_login(Some(&cube_url), None)?.as_ref()
        );
        assert_eq!(
            Some(&expected1),
            chrs_sessions
                .get_login(Some(&cube_url), Some(&expected1.username))?
                .as_ref()
        );
        assert_eq!(
            Some(&expected2),
            chrs_sessions
                .get_login(Some(&cube_url), Some(&expected2.username))?
                .as_ref()
        );
        Ok(())
    }

    #[test]
    fn test_add() -> Result<()> {
        let mut config = ChrsSessions::default();
        assert_eq!(0, config.sessions.len());
        config.add(
            CubeState {
                cube: CubeUrl::from_static("https://example.com/api/v1/"),
                username: Username::from_str("apple").unwrap(),
                token: Some("red-delicious".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            Backend::ClearText,
        )?;
        assert_eq!(1, config.sessions.len());
        assert_eq!(
            StoredToken::Text("red-delicious".to_string()),
            config
                .get_cube(
                    Some(&CubeUrl::from_static("https://example.com/api/v1/")),
                    None
                )
                .unwrap()
                .store
        );

        config.add(
            CubeState {
                cube: CubeUrl::from_static("https://example.com/api/v1/"),
                username: Username::from_str("apple").unwrap(),
                token: Some("golden-delicious".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            1,
            config.sessions.len(),
            "length is not the same after adding a Login with same address and username"
        );
        assert_eq!(
            StoredToken::Text("golden-delicious".to_string()),
            config
                .get_cube(
                    Some(&CubeUrl::from_static("https://example.com/api/v1/")),
                    None
                )
                .unwrap()
                .store
        );

        config.add(
            CubeState {
                cube: CubeUrl::from_static("https://example.com/api/v1/"),
                username: Username::from_str("pear").unwrap(),
                token: Some("yapearisachinesepear".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            2,
            config.sessions.len(),
            "length did not increase after adding a login with a different username."
        );

        config.add(
            CubeState {
                cube: CubeUrl::from_static("https://another.example.com/api/v1/"),
                username: Username::from_str("pear").unwrap(),
                token: Some("yapearisachinesepear".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            3,
            config.sessions.len(),
            "length did not increase after adding a login with a different address."
        );
        Ok(())
    }

    #[rstest]
    fn test_remove() -> Result<()> {
        let mut config = ChrsSessions::default();
        config.add(
            CubeState {
                cube: CubeUrl::from_static("https://one.example.com/api/v1/"),
                username: Username::from_str("apple").unwrap(),
                token: Some("red-delicious".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            Backend::ClearText,
        )?;
        config.add(
            CubeState {
                cube: CubeUrl::from_static("https://two.example.com/api/v1/"),
                username: Username::from_str("pear").unwrap(),
                token: Some("yapearisachinesepear".to_string()),
                current_plugin_instance_id: None,
                ui: None,
            },
            Backend::ClearText,
        )?;
        assert_eq!(
            StoredToken::Text("red-delicious".to_string()),
            config
                .get_cube(
                    Some(&CubeUrl::from_static("https://one.example.com/api/v1/")),
                    None,
                )
                .unwrap()
                .store
        );
        assert_eq!(
            StoredToken::Text("yapearisachinesepear".to_string()),
            config
                .get_cube(
                    Some(&CubeUrl::from_static("https://two.example.com/api/v1/")),
                    None
                )
                .unwrap()
                .store
        );

        let addr1 = CubeUrl::from_static("https://one.example.com/api/v1/");
        assert!(config.remove(&addr1, None));
        assert!(!config.remove(&addr1, None), "login already removed");
        assert!(config.get_cube(Some(&addr1), None).is_none());

        let addr2 = CubeUrl::from_static("https://two.example.com/api/v1/");
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

    #[rstest]
    fn test_set_plugin_instance(mut chrs_sessions: ChrsSessions) -> Result<()> {
        let cube_url = CubeUrl::from_static("https://c.example.com/api/v1/");
        let username = Username::from_static("ccccc");
        let plinst = PluginInstanceId(108);
        chrs_sessions.set_plugin_instance(&cube_url, &username, plinst);
        let actual = chrs_sessions
            .get_cube(Some(&cube_url), Some(&username))
            .unwrap();
        assert_eq!(actual.current_plugin_instance_id, Some(plinst));
        Ok(())
    }
}
