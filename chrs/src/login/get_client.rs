use crate::ChrsConfig;
use anyhow::{Context, Error, Result};
use chris::auth::CUBEAuth;
use chris::common_types::{CUBEApiUrl, Username};
use chris::ChrisClient;
use console::style;
use lazy_static::lazy_static;
use regex::Regex;

/// If `--address` is given, use it. Else, if a CUBE address appears
/// in any of `args`, use it. Else, try to get address from the saved login.
///
/// If `--password` is given, use password to get client.
/// Else, try to get saved login information from configuration file.
pub async fn get_client(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
    password: Option<String>,
    args: Vec<&str>,
) -> Result<ChrisClient> {
    let address = address.or_else(|| get_url_from(&args));
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

fn get_url_from(args: &[&str]) -> Option<CUBEApiUrl> {
    for arg in args {
        if let Some(url) = CUBE_URL_RE.find(arg) {
            if let Ok(url) = CUBEApiUrl::try_from(url.as_str()) {
                return Some(url);
            }
        }
    }
    None
}

lazy_static! {
    static ref NOT_LOGGED_IN: String = format!(
        "Not logged in. You must either provide `{}` or first run `{}`",
        style("--password").green(),
        style("chrs login").green()
    );
    static ref CUBE_URL_RE: Regex = Regex::new("^https?://.+/api/v1/").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_url_from() {
        assert_eq!(get_url_from(&vec!["one", "two"]), None);
        assert_eq!(get_url_from(&vec!["one", "http://localhost"]), None);
        assert_eq!(
            get_url_from(&vec!["one", "http://localhost/api/v1/"]),
            Some(CUBEApiUrl::try_from("http://localhost/api/v1/").unwrap())
        );
        assert_eq!(
            get_url_from(&vec!["http://localhost/api/v1/uploadedfiles/"]),
            Some(CUBEApiUrl::try_from("http://localhost/api/v1/").unwrap())
        );
        assert_eq!(
            get_url_from(&vec!["http://localhost:8000/api/v1/uploadedfiles/"]),
            Some(CUBEApiUrl::try_from("http://localhost:8000/api/v1/").unwrap())
        );
        assert_eq!(
            get_url_from(&vec!["https://cube.chrisproject.org/api/v1/uploadedfiles/"]),
            Some(CUBEApiUrl::try_from("https://cube.chrisproject.org/api/v1/").unwrap())
        );
    }
}
