use crate::credentials::Credentials;
use crate::login::state::ChrsSessions;
use color_eyre::eyre::{bail, Result};
use color_eyre::owo_colors::OwoColorize;

pub fn whoami(
    Credentials {
        cube_url,
        username,
        config_path: config_name,
        ..
    }: Credentials,
) -> Result<()> {
    let sessions = ChrsSessions::load(config_name)?;
    if let Some(login) = sessions.get_cube(cube_url.as_ref(), username.as_ref()) {
        println!(
            "Logged into ChRIS {} as user \"{}\"",
            login.cube.cyan(),
            login.username.green()
        );
        Ok(())
    } else {
        bail!("You are not logged in.")
    }
}
