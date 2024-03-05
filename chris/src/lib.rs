//! Client library for [_ChRIS_](https://chrisproject.org/) built
//! on [reqwest](https://crates.io/crates/reqwest).

mod client;
mod models;
mod requests;

// pub mod auth;
pub mod errors;
// pub mod pipeline;
mod account;
pub mod types;

pub use account::Account;
pub use client::anon::AnonChrisClient;
pub use client::authed::ChrisClient;
pub use client::base::BaseChrisClient;
pub use client::filebrowser::{FileBrowser, FileBrowserEntry};
pub use client::search::Search;
pub use client::access::{Access, RoAccess, RwAccess};
pub use models::*;

// re-export
pub use reqwest;
