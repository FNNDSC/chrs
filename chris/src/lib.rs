#[macro_use] extern crate shrinkwraprs;

pub mod auth;
mod base;
pub mod common_types;
pub mod api;
mod pagination;

pub use base::ChrisClient;

