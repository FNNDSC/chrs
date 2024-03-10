use color_eyre::eyre::{eyre, Result};
use color_eyre::owo_colors::OwoColorize;

use chris::BaseChrisClient;

use crate::arg::GivenDataNode;
use crate::credentials::Credentials;
use crate::login::state::ChrsSessions;

pub async fn cd(credentials: Credentials, given: GivenDataNode) -> Result<()> {
    let (client, old_plinst, _) = credentials.clone().get_client([given.as_arg_str()]).await?;
    if let Some(client) = client.logged_in() {
        let plinst = given.into_plinst_rw(&client, old_plinst).await?;
        let mut sessions = ChrsSessions::load(credentials.config_path.as_deref())?;
        sessions.set_plugin_instance(client.url(), client.username(), plinst.object.id);
        sessions.save(credentials.config_path)
    } else {
        Err(eyre!(
            "This command is only available for authenticated users. Try running `{}` with a username first.",
            "chrs login".bold()
        ))
    }
}
