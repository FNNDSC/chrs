//! Client library for [_ChRIS_](https://chrisproject.org/) built
//! on [reqwest](https://crates.io/crates/reqwest).

#[macro_use]
extern crate shrinkwraprs;
extern crate core;

pub mod auth;
mod client;
pub mod common_types;
mod constants;
mod models;
pub mod pipeline;
mod requests;
pub mod reqwest;

pub use crate::client::pipeline::Pipeline;
pub use client::cube::ChrisClient;
pub use auth::CUBEAuth;
pub use client::errors;
