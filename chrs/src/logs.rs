use crate::arg::GivenPluginInstance;
use crate::client::Credentials;
use color_eyre::eyre::{bail, Error, Result};

pub async fn logs(credentials: Credentials, plugin_instance: Option<String>) -> Result<()> {
    let (client, prev, _) = credentials
        .get_client(plugin_instance.as_ref().as_slice())
        .await?;
    let plugin_instance = if let Some(given) = plugin_instance {
        let given_plugin_instance = GivenPluginInstance::from(given);
        given_plugin_instance.get_using(&client, prev).await
    } else if let Some(id) = prev {
        client.get_plugin_instance(id).await.map_err(Error::new)
    } else {
        bail!("Missing operand")
    }?;
    print!("{}", plugin_instance.logs());
    Ok(())
}
