//! Shared helper functions for upload and download.

mod bytes_bar;
mod error;
mod multi_progress;

pub use bytes_bar::*;
pub use error::FileTransferError;
pub use multi_progress::*;

pub const SIZE_128_MIB: u64 = 134217728;
