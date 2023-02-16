use serde::de::DeserializeOwned;
use std::marker::PhantomData;

pub struct ConnectedModel<T: DeserializeOwned> {
    pub(crate) client: reqwest::Client,
    pub data: T,
}

pub struct ShallowModel<T: DeserializeOwned, U: reqwest::IntoUrl> {
    pub(crate) client: reqwest::Client,
    pub url: U,
    phantom: PhantomData<T>,
}

impl<T: DeserializeOwned, U: reqwest::IntoUrl> ShallowModel<T, U> {
    pub async fn get(self) -> ConnectedModel<T> {
        todo!()
    }
}
