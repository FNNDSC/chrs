//! Client library for [_ChRIS_](https://chrisproject.org/) built
//! on [reqwest](https://crates.io/crates/reqwest).

#[macro_use]
extern crate shrinkwraprs;
extern crate core;

pub mod auth;
mod client;
pub mod common_types;
mod constants;
pub mod models;
mod pagination;
pub mod pipeline;
mod requests;
pub mod reqwest;

pub use crate::client::pipeline::Pipeline;
pub use crate::client::plugin::Plugin;
pub use client::cube::ChrisClient;
pub use client::errors;
pub use client::filebrowser;
