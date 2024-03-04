//! Structs which represent *CUBE* resources that are connected/linked to other *CUBE* resources.

use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// A client to the subset of the *CUBE* API linked to by this object's generic type.
/// In less fancy speak, [LinkedModel] is a thing which can get, create, modify, or delete
/// other things or even itself.
pub struct LinkedModel<T: DeserializeOwned> {
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
    pub object: T,
}

/// You can think of [LazyLinkedModel] as a lazy [LinkedModel]: it has methods
/// for changing this resource, and can be transformed into a [LinkedModel]
/// by calling [LazyLinkedModel::get].
pub struct LazyLinkedModel<T: DeserializeOwned, U: reqwest::IntoUrl> {
    #[allow(unused)] // TODO remove me after implementing LazyLinkedModel.get
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
    pub url: U,
    phantom: PhantomData<T>,
}

impl<T: DeserializeOwned, U: reqwest::IntoUrl> LazyLinkedModel<T, U> {
    /// Make a HTTP GET request to populate the data of this object.
    pub async fn get(self) -> LinkedModel<T> {
        todo!()
    }
}

// TODO: LinkedModel should have a get method too, which "refreshes" its data.
// struct LinkedModel<T: DeserializeOwned + HasUrl>
//
// trait HasUrl {
//     pub fn url() -> reqwest::IntoUrl
// }
