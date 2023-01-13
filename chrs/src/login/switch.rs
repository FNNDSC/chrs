use crate::login::saved::SavedLogins;
use chris::common_types::{CUBEApiUrl, Username};
use console::Term;
use dialoguer::{theme::ColorfulTheme, Select};

/// Switch the preferred login. If any of `username`, `password`
/// are specified, then the process is non-interactive, and
/// selects any saved login which fits the criteria.
/// Otherwise, an interactive menu is shown where the user
/// presses arrow keys to make a selection.
pub(crate) fn switch_login(
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
) -> anyhow::Result<()> {
    let mut logins = SavedLogins::load()?;
    let max_username_len: usize = logins
        .cubes
        .iter()
        .map(|logins| logins.username.as_str().len())
        .max()
        .ok_or_else(|| anyhow::Error::msg("You are not logged in."))?;
    let selection = if let Some(selection) = noninteractive(&logins, address, username)? {
        Some(selection)
    } else {
        interactive(&logins, max_username_len)?
    };
    if let Some(selected) = selection {
        logins.set_last(selected);
        logins.store()?;
    }
    Ok(())
}

fn noninteractive(
    logins: &SavedLogins,
    address: Option<CUBEApiUrl>,
    username: Option<Username>,
) -> anyhow::Result<Option<usize>> {
    if address.is_none() && username.is_none() {
        return Ok(None);
    }
    if address.is_none() {
        return if username.is_none() {
            Ok(None)
        } else {
            Err(anyhow::Error::msg("--address is required"))
        }
    }
    let index = get_index_of(logins, &address, &username).ok_or_else(|| {
        anyhow::Error::msg(format!(
            "No login found for username={:?} address={:?}",
            username, address
        ))
    })?;
    Ok(Some(index))
}

fn get_index_of(
    logins: &SavedLogins,
    address: &Option<CUBEApiUrl>,
    username: &Option<Username>,
) -> Option<usize> {
    logins
        .get_cube(address.as_ref(), username.as_ref())
        .and_then(|login| logins.cubes.iter().position(|l| l == login))
}

fn interactive(logins: &SavedLogins, max_username_len: usize) -> anyhow::Result<Option<usize>> {
    let items: Vec<String> = logins
        .cubes
        .iter()
        .map(|login| {
            format!(
                "{:<p$}{}",
                &login.username,
                &login.address,
                p = (max_username_len + 2)
            )
        })
        .collect();
    let selection = Select::with_theme(&ColorfulTheme::default())
        .items(&items)
        .default(logins.cubes.len() - 1)
        .interact_on_opt(&Term::stderr())?;
    Ok(selection)
}
