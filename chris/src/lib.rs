//! Client library for [_ChRIS_](https://chrisproject.org/) built
//! on [reqwest](https://crates.io/crates/reqwest).

mod client;
mod requests;

pub mod auth;
pub mod constants;
pub mod errors;
pub mod models;
pub mod pipeline;
pub mod reqwest;

pub use auth::CUBEAuth;
pub use client::cube::ChrisClient;
pub use client::pipeline::Pipeline;
pub use client::*;
