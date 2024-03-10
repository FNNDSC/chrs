use crate::arg::GivenPluginInstance;
use crate::credentials::Credentials;
use crate::login::state::ChrsSessions;
use chris::types::Username;
use chris::BaseChrisClient;
use color_eyre::eyre::Result;

pub async fn cd(credentials: Credentials, given_plinst: GivenPluginInstance) -> Result<()> {
    let (client, old_plinst, _) = credentials
        .clone()
        .get_client([given_plinst.as_arg_str()])
        .await?;
    let cube_url = client.url().clone();
    let username = client.username();
    let plinst = given_plinst.get_using_either(&client, old_plinst).await?;
    let mut sessions = ChrsSessions::load(credentials.config_path.as_deref())?;
    sessions.set_plugin_instance(
        &cube_url,
        &username
            .cloned()
            .unwrap_or_else(|| Username::from_static("")),
        plinst.object.id,
    );
    sessions.save(credentials.config_path)
}
