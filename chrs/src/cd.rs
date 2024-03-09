use crate::arg::GivenPluginInstance;
use crate::client::Credentials;
use crate::login::state::ChrsSessions;
use color_eyre::eyre::Result;
use chris::BaseChrisClient;
use chris::types::Username;

pub async fn cd(credentials: Credentials, given_plinst: GivenPluginInstance) -> Result<()> {
    let (client, old_plinst, _) = credentials
        .clone()
        .get_client([given_plinst.as_arg_str()])
        .await?;
    let cube_url = client.url().clone();
    let username = client.username();
    let plinst = given_plinst.get_using_either(&client, old_plinst).await?;
    let mut sessions = ChrsSessions::load()?;
    sessions.set_plugin_instance(&cube_url, &username.cloned().unwrap_or_else(|| Username::from_static("")), plinst.object.id);
    sessions.save()
}
