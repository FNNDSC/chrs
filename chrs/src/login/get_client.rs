use crate::ChrsConfig;
use anyhow::{Context, Error, Ok, Result};
use chris::auth::CUBEAuth;
use chris::types::{CUBEApiUrl, Username};
use chris::ChrisClient;
use console::style;
use lazy_static::lazy_static;

/// If `--password` is given, use password to get client.
/// Else, try to get saved login information from configuration file.
pub async fn get_client(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>,
) -> Result<ChrisClient> {
    let (given_address, given_username, token) = match password {
        Some(given_password) => {
            let given_address = address.ok_or_else(|| Error::msg("--address is required"))?;
            let given_username = username.ok_or_else(|| Error::msg("--username is required"))?;
            let token: String = get_token(
                &Default::default(),
                &given_address,
                &given_username,
                &given_password,
            )
            .await?;
            Ok((given_address, given_username, token))
        }
        None => {
            let login = ChrsConfig::load()?
                .get_login(address.as_ref(), username.as_ref())?
                .ok_or_else(|| Error::msg(&*NOT_LOGGED_IN))?;
            Ok((login.address, login.username, login.token))
        }
    }?;
    Ok(ChrisClient::new(given_address, given_username, token).await?)
}

pub async fn get_token(
    client: &reqwest::Client,
    address: &CUBEApiUrl,
    username: &Username,
    password: &str,
) -> Result<String> {
    let account = CUBEAuth {
        client,
        url: address,
        username,
        password,
    };

    account.get_token().await.with_context(|| {
        format!(
            "Could not login to {} with username \"{}\"",
            address.as_str(),
            username.as_str()
        )
    })
}

lazy_static! {
    static ref NOT_LOGGED_IN: String = format!(
        "Not logged in. You must either provide `{}` or first run `{}`",
        style("--password").green(),
        style("chrs login").green()
    );
}
