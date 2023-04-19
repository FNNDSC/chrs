use futures::Stream;

use crate::errors::{check, CUBEError};
use crate::models::data::{PluginInstanceResponse, PluginParameter, PluginResponse};
use crate::models::linked::LinkedModel;
use serde::Serialize;

pub type Plugin = LinkedModel<PluginResponse>;

impl Plugin {
    pub async fn create_instance<T: Serialize + ?Sized>(
        &self,
        body: &T,
    ) -> Result<PluginInstanceResponse, CUBEError> {
        let res = self
            .client
            .post(self.data.instances.as_str())
            .json(body)
            .send()
            .await?;
        let data = check(res).await?.json().await?;
        Ok(data)
    }
    //
    // pub fn get_parameters(
    //     &self,
    // ) -> impl Stream<Item = Result<PluginParameter, reqwest::Error>> + '_ {
    //     paginate(&self.client, Some(self.plugin.parameters.clone()))
    // }
}
