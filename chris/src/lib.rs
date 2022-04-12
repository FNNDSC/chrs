#[macro_use]
extern crate shrinkwraprs;
extern crate core;

pub mod api;
pub mod auth;
mod client;
pub mod common_types;
mod pagination;
pub mod pipeline;

pub use client::{ChrisClient, CUBEError, UploadError};
