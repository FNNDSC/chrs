#[macro_use]
extern crate shrinkwraprs;

pub mod api;
pub mod auth;
mod base;
pub mod common_types;
mod pagination;
mod pipelines;

pub use base::ChrisClient;
