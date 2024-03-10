use color_eyre::eyre::Result;

use crate::arg::GivenPluginInstanceOrPath;
use crate::credentials::Credentials;

pub async fn logs(
    credentials: Credentials,
    plugin_instance: GivenPluginInstanceOrPath,
) -> Result<()> {
    let (client, old, _) = credentials
        .get_client([plugin_instance.as_arg_str()])
        .await?;
    let logs = plugin_instance.get_using_either(&client, old).await?.logs();
    print!("{}", logs);
    Ok(())
}
