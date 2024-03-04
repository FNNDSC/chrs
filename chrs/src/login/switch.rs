use super::state::ChrsSessions;
use crate::get_client::Credentials;
use chris::types::{CubeUrl, Username};
use color_eyre::eyre::{Error, Result};
use color_eyre::owo_colors::OwoColorize;
use dialoguer::console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

/// Switch the preferred login. If any of `username`, `password`
/// are specified, then the process is non-interactive, and
/// selects any saved login which fits the criteria.
/// Otherwise, an interactive menu is shown where the user
/// presses arrow keys to make a selection.
pub(crate) fn switch_login(
    Credentials {
        cube_url, username, ..
    }: Credentials,
) -> Result<()> {
    let mut logins = ChrsSessions::load()?;

    if logins.sessions.len() == 1 {
        let login = &logins.sessions[0];
        println!(
            "Only one login found. Logged into ChRIS {} as user \"{}\"",
            login.cube, login.username
        );
        return Ok(());
    }

    let max_username_len: usize = logins
        .sessions
        .iter()
        .map(|logins| logins.username.as_str().len())
        .max()
        .ok_or_else(|| Error::msg("You are not logged in."))?;
    let selection = if let Some(selection) = noninteractive(&logins, cube_url, username)? {
        Some(selection)
    } else {
        interactive(&logins, max_username_len)?
    };
    if let Some(selected) = selection {
        logins.set_last(selected);
        logins.save()?;
    }
    Ok(())
}

fn noninteractive(
    logins: &ChrsSessions,
    cube_url: Option<CubeUrl>,
    username: Option<Username>,
) -> Result<Option<usize>> {
    if cube_url.is_none() && username.is_none() {
        return Ok(None);
    }
    if cube_url.is_none() {
        return if username.is_none() {
            Ok(None)
        } else {
            Err(Error::msg("--cube is required"))
        };
    }
    let index = get_index_of(logins, &cube_url, &username).ok_or_else(|| {
        Error::msg(format!(
            "No login found for username={:?} cube={:?}",
            username.green(),
            cube_url.cyan()
        ))
    })?;
    Ok(Some(index))
}

fn get_index_of(
    logins: &ChrsSessions,
    address: &Option<CubeUrl>,
    username: &Option<Username>,
) -> Option<usize> {
    logins
        .get_cube(address.as_ref(), username.as_ref())
        .and_then(|login| logins.sessions.iter().position(|l| l == login))
}

fn interactive(logins: &ChrsSessions, max_username_len: usize) -> Result<Option<usize>> {
    let max_username_len = std::cmp::max(max_username_len, "(anonymous)".len());
    let items: Vec<String> = logins
        .sessions
        .iter()
        .map(|login| {
            format!(
                "{:<p$}{}",
                if login.username.as_str().is_empty() {
                    "(anonymous)"
                } else {
                    &login.username.as_str()
                },
                &login.cube,
                p = (max_username_len + 2)
            )
        })
        .collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(logins.sessions.len() - 1)
        .interact_on_opt(&Term::stderr())?;
    Ok(selection)
}
