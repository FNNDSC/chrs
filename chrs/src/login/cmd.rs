use super::prompt::{prompt_if_missing, prompt_if_missing_password};
use super::state::ChrsSessions;
use super::store;
use crate::get_client::Credentials;
use chris::{
    types::{CubeUrl, Username},
    Account, AnonChrisClient, ChrisClient,
};
use color_eyre::eyre::{bail, Context, Result};
use color_eyre::owo_colors::OwoColorize;

pub async fn login(
    Credentials {
        cube_url,
        username,
        password,
        token,
        ..
    }: Credentials,
    backend: store::Backend,
    password_from_stdin: bool,
) -> Result<()> {
    if password.is_some() && password_from_stdin {
        bail!(
            "Options {} and {} may not be used together.",
            "--password".bold(),
            "--password-stdin".bold()
        );
    }

    let mut config = ChrsSessions::load()?;
    let cube = prompt_if_missing(cube_url, "ChRIS API address")?;
    let username = prompt_if_missing(username, "username")?;

    let token = if username.as_str().is_empty() {
        login_anonymous(&cube).await
    } else if let Some(token) = token {
        login_with_token(&cube, &username, &token).await
    } else {
        let password = prompt_if_missing_password(password, "password", password_from_stdin)?;
        login_with_password(&cube, &username, &password).await
    }?;

    let login = store::CubeState {
        cube,
        token,
        username,
        current_plugin_instance_id: None,
    };

    config.add(login, backend)?;
    config.save()
}

/// Contact CUBE just to make sure CUBE is reachable.
async fn login_anonymous(cube_url: &CubeUrl) -> Result<Option<String>> {
    AnonChrisClient::build(cube_url.clone())?.connect().await?;
    Ok(None)
}

/// Login to CUBE by getting a token using a password.
async fn login_with_password(
    cube_url: &CubeUrl,
    username: &Username,
    password: &str,
) -> Result<Option<String>> {
    let account = Account {
        client: Default::default(),
        url: cube_url,
        username,
        password,
    };

    let token = account.get_token().await.wrap_err("Could not log in")?;
    Ok(Some(token))
}

/// Verify token works for the CUBE.
async fn login_with_token(
    cube_url: &CubeUrl,
    username: &Username,
    token: &str,
) -> Result<Option<String>> {
    ChrisClient::build(cube_url.clone(), username.clone(), token)?
        .connect()
        .await
        .wrap_err_with(|| format!("Invalid token for {cube_url}"))?;
    Ok(Some(token.to_string()))
}

pub fn logout(
    Credentials {
        cube_url, username, ..
    }: Credentials,
) -> Result<()> {
    let mut config = ChrsSessions::load()?;
    if let Some(url) = cube_url {
        let removed = match username {
            Some(u) => config.remove(&url, Some(&u)),
            None => config.remove(&url, None),
        };
        if !removed {
            bail!("Not logged in.");
        }
    } else if !config.clear() {
        bail!("Not logged in.");
    }
    config.save()
}
