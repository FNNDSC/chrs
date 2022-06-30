#[macro_use]
extern crate shrinkwraprs;
extern crate core;

pub mod api;
pub mod auth;
mod client;
pub mod common_types;
mod constants;
mod pagination;
pub(crate) mod pipeline;

pub use client::cube::ChrisClient;
pub use client::errors;
pub use client::filebrowser;
pub use client::pipeline::Pipeline;
