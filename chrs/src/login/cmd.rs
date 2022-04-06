use crate::config;
use crate::login::get_client::get_token;
use crate::login::prompt::{prompt_if_missing, prompt_if_missing_password};
use crate::login::tokenstore;
use anyhow::{bail, Result};
use chris::types::{CUBEApiUrl, Username};

pub async fn login(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>,
    backend: tokenstore::Backend,
    password_from_stdin: &bool,
) -> Result<()> {
    if password.is_some() && *password_from_stdin {
        bail!("Options --password and --password-stdin may not be used together.");
    }

    let mut config = config::ChrsConfig::load()?;
    let given_address = prompt_if_missing(address, "ChRIS API address")?;
    let given_username = prompt_if_missing(username, "username")?;
    let given_password = prompt_if_missing_password(password, "password", password_from_stdin)?;

    let token = get_token(
        &Default::default(),
        &given_address,
        &given_username,
        &given_password,
    )
    .await?;

    let login = tokenstore::Login {
        address: given_address,
        username: given_username,
        token,
    };
    config.add(login, backend)?;
    config.store()
}

pub fn logout(address: Option<CUBEApiUrl>, username: Option<Username>) -> anyhow::Result<()> {
    let mut config = config::ChrsConfig::load()?;
    if let Some(given_address) = address {
        let removed = match username {
            Some(u) => config.remove(&given_address, Some(&u)),
            None => config.remove(&given_address, None),
        };
        if !removed {
            bail!("Not logged in.");
        }
    } else if !config.clear() {
        bail!("Not logged in.");
    }
    config.store()
}
