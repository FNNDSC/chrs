use crate::login::prompt::{prompt_if_missing, prompt_if_missing_password};
use crate::login::saved;
use crate::login::tokenstore;
use anyhow::{bail, Context, Result};
use chris::auth::CUBEAuth;
use chris::common_types::{CUBEApiUrl, Username};

pub async fn login(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>,
    backend: tokenstore::Backend,
    password_from_stdin: bool,
) -> Result<()> {
    if password.is_some() && password_from_stdin {
        bail!("Options --password and --password-stdin may not be used together.");
    }

    let mut config = saved::SavedLogins::load()?;
    let account = CUBEAuth {
        client: &Default::default(),
        url: prompt_if_missing(address, "ChRIS API address")?,
        username: prompt_if_missing(username, "username")?,
        password: prompt_if_missing_password(password, "password", password_from_stdin)?,
    };

    let login = tokenstore::Login {
        token: account.get_token().await.context("Could not log in")?,
        address: account.url,
        username: account.username,
    };
    config.add(login, backend)?;
    config.store()
}

pub fn logout(address: Option<CUBEApiUrl>, username: Option<Username>) -> anyhow::Result<()> {
    let mut config = saved::SavedLogins::load()?;
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
