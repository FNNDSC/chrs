// use serde::Deserialize;

use crate::types::Username;

#[derive(Debug)]
pub struct ChrisClient {
    pub username: Username,
    _token: String,
    // links: CUBELinks,
}

impl ChrisClient {
    pub async fn new(username: Username, token: String) -> Self {
        ChrisClient {
            username,
            _token: token,
        }
    }
}
