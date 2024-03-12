use super::state::ChrsSessions;
use chris::types::{CubeUrl, PluginInstanceId, Username};
use color_eyre::eyre;
use std::path::PathBuf;

pub fn set_cd(
    cube_url: &CubeUrl,
    username: &Username,
    id: PluginInstanceId,
    config_path: Option<PathBuf>,
) -> eyre::Result<()> {
    let mut sessions = ChrsSessions::load(config_path.as_deref())?;
    if sessions.set_plugin_instance(cube_url, username, id) {
        sessions.save(config_path.as_deref())?;
    }
    Ok(())
}
