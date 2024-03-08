use color_eyre::eyre::Result;

use crate::arg::GivenPluginInstance;
use crate::client::Credentials;

pub async fn logs(credentials: Credentials, plugin_instance: GivenPluginInstance) -> Result<()> {
    let (client, old, _) = credentials
        .get_client([plugin_instance.as_arg_str()])
        .await?;
    let logs = plugin_instance.get_using(&client, old).await?.logs();
    print!("{}", logs);
    Ok(())
}
