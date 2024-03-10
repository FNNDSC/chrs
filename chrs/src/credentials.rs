use color_eyre::eyre;
use color_eyre::eyre::{eyre, Context};
use color_eyre::owo_colors::OwoColorize;
use reqwest_middleware::Middleware;
use reqwest_retry::{
    policies::ExponentialBackoff, RetryTransientMiddleware, Retryable, RetryableStrategy,
};
use std::path::PathBuf;

use chris::reqwest::Response;
use chris::types::{CubeUrl, PluginInstanceId, Username};
use chris::{Account, AnonChrisClient, ChrisClient, EitherClient};

use crate::login::state::ChrsSessions;
use crate::login::store::CubeState;
use crate::login::UiUrl;

/// A dummy value to provide to [Credentials::get_client]
pub const NO_ARGS: [&str; 0] = [];

/// Command-line options of `chrs` which are relevant to identifying the user session
/// and obtaining a client object.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub cube_url: Option<CubeUrl>,
    pub username: Option<Username>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub retries: Option<u32>,
    pub ui: Option<UiUrl>,
    /// Name of configuration file.
    ///
    /// - `None`: use default configuration file (for main use)
    /// - `Some(_)`: custom configuration file (for testing purposes only)
    pub config_path: Option<PathBuf>,
}

impl Credentials {
    /// If `--cube` is given, use it. Else, if a CUBE address appears
    /// in any of `args`, use it. Else, try to get address from the saved login.
    ///
    /// If `--password` is given, use password to get client.
    /// Else, try to get saved login information from configuration file.
    pub async fn get_client(
        self,
        args: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> eyre::Result<(EitherClient, Option<PluginInstanceId>, Option<UiUrl>)> {
        let Credentials {
            cube_url,
            username,
            password,
            token,
            retries,
            ui,
            config_path: config_name,
        } = self;
        let retry_middleware = retries.map(retry_strategy);
        if let (Some(url), Some(token), Some(username)) =
            (cube_url.as_ref(), token, username.as_ref())
        {
            let builder = ChrisClient::build(url.clone(), username.clone(), token)?;
            let builder = if let Some(middleware) = retry_middleware {
                builder.with(middleware)
            } else {
                builder
            };
            return builder
                .connect()
                .await
                .map(EitherClient::LoggedIn)
                .map(|c| (c, None, ui))
                .map_err(eyre::Error::new);
        }
        if let Some(password) = password {
            get_client_with_password(cube_url, username, password, args, retry_middleware)
                .await
                .map(EitherClient::LoggedIn)
                .map(|c| (c, None, ui))
        } else {
            get_client_from_state(cube_url, username, ui, args, retry_middleware, config_name).await
        }
    }
}

/// Get an authenticated _ChRIS_ client using the provided options.
async fn get_client_with_password(
    cube_url: Option<CubeUrl>,
    username: Option<Username>,
    password: String,
    args: impl IntoIterator<Item = impl AsRef<str>>,
    retry_middleware: Option<impl Middleware>,
) -> eyre::Result<ChrisClient> {
    let url = cube_url
        .or_else(|| first_cube_urllike(args))
        .ok_or_else(|| eyre!("--cube is required"))?;
    let username = username.ok_or_else(|| eyre!("--username is required"))?;
    let account = Account {
        client: Default::default(),
        url: &url,
        username: &username,
        password: &password,
    };
    let token = account
        .get_token()
        .await
        .map_err(|e| handle_error(e, &url))?;
    let client = if let Some(middleware) = retry_middleware {
        ChrisClient::build(url, username, token)?
            .with(middleware)
            .connect()
            .await
    } else {
        ChrisClient::build(url, username, token)?.connect().await
    }?;
    Ok(client)
}

/// Get the client, using the previously saved config file if needed.
async fn get_client_from_state(
    cube_url: Option<CubeUrl>,
    username: Option<Username>,
    ui: Option<UiUrl>,
    args: impl IntoIterator<Item = impl AsRef<str>>,
    retry_middleware: Option<impl Middleware>,
    config_path: Option<PathBuf>,
) -> eyre::Result<(EitherClient, Option<PluginInstanceId>, Option<UiUrl>)> {
    let url = cube_url.clone().or_else(|| first_cube_urllike(args));
    let login = ChrsSessions::load(config_path)?
        .get_login(url.as_ref(), username.as_ref())?
        .or_else(|| {
            // If --cube is not given, no matching login found, but a URL is found from the
            // positional args, try doing an anonymous login.
            cube_url.or(url).map(|cube| CubeState {
                cube,
                username: Username::from_static(""),
                token: None,
                current_plugin_instance_id: None,
                ui: ui.clone(),
            })
        })
        .ok_or_else(|| {
            eyre!(
                "Not logged in. Either use the {} option, or run `{}`",
                "--cube".bold(),
                "chrs login".bold()
            )
        })?;
    let client = if login.username.as_str().is_empty() {
        get_anon_client(login.cube, retry_middleware).await
    } else {
        get_authed_client(login.cube, login.username, login.token, retry_middleware).await
    }?;
    Ok((client, login.current_plugin_instance_id, ui.or(login.ui)))
}

async fn get_anon_client(
    cube_url: CubeUrl,
    retry_middleware: Option<impl Middleware>,
) -> color_eyre::Result<EitherClient> {
    let client = if let Some(middleware) = retry_middleware {
        AnonChrisClient::build(cube_url)?
            .with(middleware)
            .connect()
            .await
    } else {
        AnonChrisClient::build(cube_url)?.connect().await
    }?;
    Ok(EitherClient::Anon(client))
}

async fn get_authed_client(
    cube_url: CubeUrl,
    username: Username,
    token: Option<String>,
    retry_middleware: Option<impl Middleware>,
) -> color_eyre::Result<EitherClient> {
    let token = token.ok_or_else(|| {
        eyre!(
            "The saved token is invalid, please run `{}`",
            format!(
                "chrs logout --cube \"{}\" --username \"{}\"",
                &cube_url, &username
            )
            .bold()
        )
    })?;
    let result = if let Some(middleware) = retry_middleware {
        ChrisClient::build(cube_url.clone(), username.clone(), token)?
            .with(middleware)
            .connect()
            .await
    } else {
        ChrisClient::build(cube_url.clone(), username.clone(), token)?
            .connect()
            .await
    };
    result.map(EitherClient::LoggedIn).wrap_err_with(|| {
        format!(
            "Could not log in. The saved token might have expired, please run {}",
            format!(
                "chrs logout --cube \"{}\" --username \"{}\"",
                &cube_url, &username
            )
            .bold()
        )
    })
}

fn handle_error(error: chris::reqwest::Error, url: &CubeUrl) -> eyre::Error {
    if let Some(code) = error.status() {
        if code == chris::reqwest::StatusCode::UNAUTHORIZED {
            eyre::Error::msg("Incorrect login")
        } else {
            eyre::Error::msg(format!("HTTP status code: {code}"))
        }
    } else {
        eyre::Error::msg(format!("Failed HTTP request to {url}"))
    }
}

fn first_cube_urllike(args: impl IntoIterator<Item = impl AsRef<str>>) -> Option<CubeUrl> {
    args.into_iter().filter_map(parse_cube_url_from).next()
}

fn parse_cube_url_from(arg: impl AsRef<str>) -> Option<CubeUrl> {
    arg.as_ref()
        .split_once("/api/v1/")
        .map(|(url, _path)| format!("{url}/api/v1/"))
        .and_then(|url| CubeUrl::new(url).ok())
}

fn retry_strategy(retries: u32) -> impl reqwest_middleware::Middleware {
    let policy = ExponentialBackoff::builder().build_with_max_retries(retries);
    RetryTransientMiddleware::new_with_policy_and_strategy(policy, RetryStrategy)
}

/// - Client errors are fatal
/// - Everything else can be retried
struct RetryStrategy;
impl RetryableStrategy for RetryStrategy {
    fn handle(&self, res: &Result<Response, reqwest_middleware::Error>) -> Option<Retryable> {
        if let Ok(response) = res {
            if response.status().is_server_error() {
                Some(Retryable::Transient)
            } else if response.status().is_client_error() {
                Some(Retryable::Fatal)
            } else {
                None
            }
        } else {
            Some(Retryable::Transient)
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;

    use super::*;

    #[rstest]
    #[case([], None)]
    #[case(["hello"], None)]
    #[case(["https://example.org/api/v1/"], Some("https://example.org/api/v1/"))]
    #[case(["hello", "https://example.org/api/v1/"], Some("https://example.org/api/v1/"))]
    #[case(["https://example.org/api/v1/files/113/"], Some("https://example.org/api/v1/"))]
    #[case(["https://example.org/api/v1/plugins/4/"], Some("https://example.org/api/v1/"))]
    fn test_first_cube_urllike<'a>(
        #[case] args: impl IntoIterator<Item = &'a str>,
        #[case] expected: Option<&'static str>,
    ) {
        assert_eq!(
            first_cube_urllike(args),
            expected.map(|s| CubeUrl::from_static(s))
        );
    }
}
