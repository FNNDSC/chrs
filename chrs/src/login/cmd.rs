use crate::config;
use crate::login::tokenstore;
use crate::login::tokenstore::SavedCubeAuth;
use anyhow::{Context, Ok, Result};
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
            given_address.to_string(),
            given_username.to_string()
        )
    })?;
    let login = tokenstore::Login {
        address: given_address.to_string(),
        username: given_username.to_string(),
        token,
    };
    config.add(login, backend)?;
    config.store()
}
