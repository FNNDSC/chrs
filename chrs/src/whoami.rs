use crate::login::state::ChrsSessions;
use chris::types::{CubeUrl, Username};
use color_eyre::eyre::{bail, Result};
use owo_colors::OwoColorize;

pub fn whoami(cube_url: Option<CubeUrl>, username: Option<Username>) -> Result<()> {
    let sessions = ChrsSessions::load()?;
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
