//! Structs which represent *CUBE* resources that are connected/linked to other *CUBE* resources.

use crate::client::access::Access;
use crate::errors::{check, CubeError};
use crate::types::ItemUrl;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use crate::{RoAccess, RwAccess};

/// A client to the subset of the *CUBE* API linked to by this object's generic type.
/// In less fancy speak, [LinkedModel] is a thing which can get, create, modify, or delete
/// other things or even itself.
pub struct LinkedModel<T: DeserializeOwned, A: Access> {
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
    pub object: T,
    pub(crate) phantom: PhantomData<A>,
}

impl<T: DeserializeOwned> From<LinkedModel<T, RwAccess>> for LinkedModel<T, RoAccess> {
    fn from(value: LinkedModel<T, RwAccess>) -> LinkedModel<T, RoAccess> {
        LinkedModel {
            client: value.client,
            object: value.object,
            phantom: Default::default()
        }
    }
}

/// You can think of [LazyLinkedModel] as a lazy [LinkedModel]: it has methods
/// for changing this resource, and can be transformed into a [LinkedModel]
/// by calling [LazyLinkedModel::get].
pub struct LazyLinkedModel<'a, T: DeserializeOwned, A: Access> {
    pub url: &'a ItemUrl,
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
    pub(crate) phantom: PhantomData<(T, A)>,
}

impl<T: DeserializeOwned, A: Access> LazyLinkedModel<'_, T, A> {
    pub async fn get(self) -> Result<LinkedModel<T, A>, CubeError> {
        let res = self.client.get(self.url.as_str()).send().await?;
        let data = check(res).await?.json().await?;
        Ok(LinkedModel {
            object: data,
            client: self.client,
            phantom: Default::default(),
        })
    }
}

// Future work:
// LinkedModel should have a get method too, which "refreshes" its data.
// struct LinkedModel<T: DeserializeOwned + HasUrl>
//
// trait HasUrl {
//     pub fn url() -> reqwest::IntoUrl
// }
