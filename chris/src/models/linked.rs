//! Structs which represent *CUBE* resources that are connected/linked to other *CUBE* resources.

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::client::access::Access;
use crate::errors::{check, CubeError};
use crate::search::SearchBuilder;
use crate::types::{CollectionUrl, ItemUrl};
use crate::{RoAccess, RwAccess};

/// A client to the subset of the *CUBE* API linked to by this object's generic type.
/// In less fancy speak, [LinkedModel] is a thing which can get, create, modify, or delete
/// other things or even itself.
pub struct LinkedModel<T: DeserializeOwned, A: Access> {
    pub object: T,
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
    pub(crate) phantom: PhantomData<A>,
}

impl<T: DeserializeOwned, A: Access> LinkedModel<T, A> {
    /// Get a lazy object of a link
    pub(crate) fn get_lazy<'a, R: DeserializeOwned>(
        &'a self,
        url: &'a ItemUrl,
    ) -> LazyLinkedModel<R, A> {
        LazyLinkedModel {
            url,
            client: &self.client,
            phantom: Default::default(),
        }
    }

    /// Get items in a collection
    pub(crate) fn get_collection<R: DeserializeOwned>(
        &self,
        url: &CollectionUrl,
    ) -> SearchBuilder<R, A> {
        SearchBuilder::collection(self.client.clone(), url.clone())
    }

    /// HTTP put request
    pub(crate) async fn put<S: Serialize + ?Sized>(
        &self,
        url: &ItemUrl,
        data: &S,
    ) -> Result<Self, CubeError> {
        let res = self.client.put(url.as_str()).json(data).send().await?;
        let data = check(res).await?.json().await?;
        Ok(Self {
            client: self.client.clone(),
            object: data,
            phantom: Default::default(),
        })
    }

    /// HTTP POST request
    pub(crate) async fn post<S: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        url: &CollectionUrl,
        data: &S,
    ) -> Result<LinkedModel<R, A>, CubeError> {
        let res = if std::mem::size_of_val(data) == 0 {
            // workaround for https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/382
            self.client.post(url.as_str()).send().await
        } else {
            self.client.post(url.as_str()).json(data).send().await
        }?;
        let data = check(res).await?.json().await?;
        Ok(LinkedModel {
            client: self.client.clone(),
            object: data,
            phantom: Default::default(),
        })
    }
}

impl<T: DeserializeOwned> From<LinkedModel<T, RwAccess>> for LinkedModel<T, RoAccess> {
    fn from(value: LinkedModel<T, RwAccess>) -> LinkedModel<T, RoAccess> {
        LinkedModel {
            client: value.client,
            object: value.object,
            phantom: Default::default(),
        }
    }
}

/// You can think of [LazyLinkedModel] as a lazy [LinkedModel]: it has methods
/// for changing this resource, and can be transformed into a [LinkedModel]
/// by calling [LazyLinkedModel::get].
pub struct LazyLinkedModel<'a, T: DeserializeOwned, A: Access> {
    pub url: &'a ItemUrl,
    pub(crate) client: &'a reqwest_middleware::ClientWithMiddleware,
    pub(crate) phantom: PhantomData<(T, A)>,
}

impl<T: DeserializeOwned, A: Access> LazyLinkedModel<'_, T, A> {
    pub async fn get(self) -> Result<LinkedModel<T, A>, CubeError> {
        let res = self.client.get(self.url.as_str()).send().await?;
        let data = check(res).await?.json().await?;
        Ok(LinkedModel {
            object: data,
            client: self.client.clone(),
            phantom: Default::default(),
        })
    }

    /// HTTP put request
    pub(crate) async fn put<S: Serialize + ?Sized>(
        &self,
        data: &S,
    ) -> Result<LinkedModel<T, A>, CubeError> {
        let res = self.client.put(self.url.as_str()).json(data).send().await?;
        let data = check(res).await?.json().await?;
        Ok(LinkedModel {
            client: self.client.clone(),
            object: data,
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
