use crate::config;
use crate::login::tokenstore;
use anyhow::{bail, Context, Ok, Result};
use chris::auth::CUBEAuth;
use chris::types::{CUBEApiUrl, Username};

pub async fn login(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>,
    backend: tokenstore::Backend,
) -> Result<()> {
    let mut config = config::ChrsConfig::load()?;
    let given_address = address.context("--address is required")?;
    let given_username = username.context("--username is required")?;
    let given_password = password.context("--password is required")?;

    let account = CUBEAuth {
        client: &Default::default(),
        url: &given_address,
        username: &given_username,
        password: given_password.as_str(),
    };

    let token = account.get_token().await.with_context(|| {
        format!(
            "Could not login to {} with username \"{}\"",
            given_address.as_str(),
            given_username.as_str()
        )
    })?;
    let login = tokenstore::Login {
        address: given_address,
        username: given_username,
        token,
    };
    config.add(login, backend)?;
    config.store()
}

pub fn logout(address: Option<CUBEApiUrl>, username: Option<Username>) -> Result<()> {
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
    config.store()?;
    Ok(())
}
