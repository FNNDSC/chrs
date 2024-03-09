use crate::login::state::ChrsSessions;
use crate::login::store::CubeState;
use crate::login::UiUrl;
use chris::errors::CubeError;
use chris::reqwest::Response;
use chris::types::{CubeUrl, FeedId, PluginInstanceId, Username};
use chris::{
    Account, AnonChrisClient, BaseChrisClient, ChrisClient, FeedRo, PluginInstanceRo, RoAccess,
};
use color_eyre::eyre::{bail, eyre, Context, Error, OptionExt};
use color_eyre::owo_colors::OwoColorize;
use futures::TryStreamExt;
use itertools::Itertools;
use reqwest_middleware::Middleware;
use reqwest_retry::{
    policies::ExponentialBackoff, RetryTransientMiddleware, Retryable, RetryableStrategy,
};

/// A client which accesses read-only APIs only.
/// It may use authorization, in which case it is able to read private collections.
pub type RoClient = Box<dyn BaseChrisClient<RoAccess>>;

/// A dummy value to provide to [Credentials::get_client]
pub const NO_ARGS: [&str; 0] = [];

/// Either an anonymous client or a logged in user.
pub enum Client {
    Anon(AnonChrisClient),
    LoggedIn(ChrisClient),
}

impl Client {
    /// Use this client for public read-only access only.
    pub fn into_ro(self) -> RoClient {
        match self {
            Self::Anon(c) => Box::new(c),
            Self::LoggedIn(c) => Box::new(c.into_ro()),
        }
    }

    pub fn url(&self) -> &CubeUrl {
        match self {
            Self::Anon(c) => c.url(),
            Self::LoggedIn(c) => c.url(),
        }
    }

    pub fn username(&self) -> Username {
        match self {
            Self::Anon(_) => Username::from_static(""),
            Self::LoggedIn(c) => c.username().clone(),
        }
    }

    pub async fn get_plugin_instance(
        &self,
        id: PluginInstanceId,
    ) -> Result<PluginInstanceRo, CubeError> {
        match self {
            Self::Anon(c) => c.get_plugin_instance(id).await,
            Self::LoggedIn(c) => c.get_plugin_instance(id).await.map(|p| p.into()),
        }
    }

    pub async fn get_feed(&self, id: FeedId) -> Result<FeedRo, CubeError> {
        match self {
            Self::Anon(c) => c.get_feed(id).await,
            Self::LoggedIn(c) => c.get_feed(id).await.map(|f| f.into()),
        }
    }

    pub async fn get_feed_by_name(&self, name: &str) -> color_eyre::Result<FeedRo> {
        let feeds: Vec<_> = match self {
            Self::Anon(c) => {
                let query = c.public_feeds().name(name).page_limit(10).max_items(10);
                query.search().stream_connected().try_collect().await
            }
            Self::LoggedIn(c) => {
                // need to get both public feeds and private feeds
                // https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530
                let private_query = c.feeds().name(name).page_limit(10).max_items(10);
                let private_feeds: Vec<_> = private_query
                    .search()
                    .stream_connected()
                    .map_ok(|f| f.into())
                    .try_collect()
                    .await?;
                if private_feeds.is_empty() {
                    let public_feeds_query =
                        c.public_feeds().name(name).page_limit(10).max_items(10);
                    public_feeds_query
                        .search()
                        .stream_connected()
                        .try_collect()
                        .await
                } else {
                    Ok(private_feeds)
                }
            }
        }?;
        if feeds.len() > 1 {
            bail!(
                "More than one feed found: {}",
                feeds
                    .iter()
                    .map(|f| format!("feed/{}", f.object.id.0))
                    .join(" ")
            )
        }
        feeds.into_iter().next().ok_or_eyre("Feed not found")
    }
}

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
    ) -> color_eyre::Result<(Client, Option<PluginInstanceId>, Option<UiUrl>)> {
        let Credentials {
            cube_url,
            username,
            password,
            token,
            retries,
            ui,
        } = self;
        if token.is_some() {
            eprintln!("{}", "warning: --token was ignored".dimmed());
        }
        let retry_middleware = retries.map(retry_strategy);
        if let Some(password) = password {
            get_client_with_password(cube_url, username, password, args, retry_middleware)
                .await
                .map(Client::LoggedIn)
                .map(|c| (c, None, ui))
        } else {
            get_client_from_state(cube_url, username, ui, args, retry_middleware).await
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
) -> std::result::Result<ChrisClient, Error> {
    let url = cube_url
        .or_else(|| first_cube_urllike(args))
        .ok_or_else(|| Error::msg("--cube is required"))?;
    let username = username.ok_or_else(|| Error::msg("--username is required"))?;
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
) -> Result<(Client, Option<PluginInstanceId>, Option<UiUrl>), Error> {
    let url = cube_url.clone().or_else(|| first_cube_urllike(args));
    let login = ChrsSessions::load()?
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
            Error::msg(format!(
                "Not logged in. Either use the {} option, or run `{}`",
                "--cube".bold(),
                "chrs login".bold()
            ))
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
) -> color_eyre::Result<Client> {
    let client = if let Some(middleware) = retry_middleware {
        AnonChrisClient::build(cube_url)?
            .with(middleware)
            .connect()
            .await
    } else {
        AnonChrisClient::build(cube_url)?.connect().await
    }?;
    Ok(Client::Anon(client))
}

async fn get_authed_client(
    cube_url: CubeUrl,
    username: Username,
    token: Option<String>,
    retry_middleware: Option<impl Middleware>,
) -> color_eyre::Result<Client> {
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
    result.map(Client::LoggedIn).wrap_err_with(|| {
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

fn handle_error(error: chris::reqwest::Error, url: &CubeUrl) -> Error {
    if let Some(code) = error.status() {
        if code == chris::reqwest::StatusCode::UNAUTHORIZED {
            Error::msg("Incorrect login")
        } else {
            Error::msg(format!("HTTP status code: {code}"))
        }
    } else {
        Error::msg(format!("Failed HTTP request to {url}"))
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
    fn handle(
        &self,
        res: &std::result::Result<Response, reqwest_middleware::Error>,
    ) -> Option<Retryable> {
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
    use super::*;
    use rstest::*;

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
