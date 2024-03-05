use crate::arg::GivenPluginInstance;
use crate::get_client::Credentials;
use crate::login::state::ChrsSessions;
use color_eyre::eyre::Result;

pub async fn cd(credentials: Credentials, given_plinst: String) -> Result<()> {
    let (client, old_plinst) = credentials.clone().get_client([&given_plinst]).await?;
    let given_plinst = GivenPluginInstance::from(given_plinst);
    let cube_url = client.url().clone();
    let username = client.username();
    let plinst = given_plinst.get_using(&client, old_plinst).await?;
    let mut sessions = ChrsSessions::load()?;
    sessions.set_plugin_instance(&cube_url, &username, plinst.id);
    sessions.save()
}
