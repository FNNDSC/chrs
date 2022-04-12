use crate::ChrsConfig;
use anyhow::{Context, Error, Result};
use chris::auth::CUBEAuth;
use chris::common_types::{CUBEApiUrl, Username};
use chris::ChrisClient;
use console::style;
use lazy_static::lazy_static;

/// If `--password` is given, use password to get client.
/// Else, try to get saved login information from configuration file.
pub async fn get_client(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>,
    // TODO we should also consider if any positional URL has an address as well.
) -> Result<ChrisClient> {
    match password {
        Some(given_password) => {
            let given_address = address.ok_or_else(|| Error::msg("--address is required"))?;
            let given_username = username.ok_or_else(|| Error::msg("--username is required"))?;
            let account = CUBEAuth {
                client: &Default::default(),
                url: given_address,
                username: given_username,
                password: given_password,
            };
            account.into_client().await.context("Password incorrect")
        }
        None => {
            let login = ChrsConfig::load()?
                .get_login(address.as_ref(), username.as_ref())?
                .ok_or_else(|| Error::msg(&*NOT_LOGGED_IN))?;
            login.into_client().await.with_context(|| {
                format!(
                    "Could not log in. \
                Your token might have expired, please run {}",
                    style("chrs logout").bold()
                )
            })
        }
    }
}

lazy_static! {
    static ref NOT_LOGGED_IN: String = format!(
        "Not logged in. You must either provide `{}` or first run `{}`",
        style("--password").green(),
        style("chrs login").green()
    );
}
