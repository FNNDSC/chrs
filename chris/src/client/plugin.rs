use crate::api::*;
use crate::client::plugininstance::PluginInstance;
use crate::errors::{check, CUBEError};
use reqwest::Client;
use serde::Serialize;

pub struct Plugin {
    client: Client,
    pub plugin: PluginResponse,
}

impl Plugin {
    pub(crate) fn new(client: Client, res: PluginResponse) -> Self {
        Plugin {
            client,
            plugin: res,
        }
    }

    pub async fn create_instance<T: Serialize + ?Sized>(
        &self,
        body: &T,
    ) -> Result<PluginInstance, CUBEError> {
        let res = self
            .client
            .post(self.plugin.instances.as_str())
            .json(body)
            .send()
            .await?;
        let data = check(res).await?.json().await?;
        Ok(PluginInstance::new(self.client.clone(), data))
    }
}
