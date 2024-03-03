//! Representations of data from *CUBE*.
//!
//! ## How It Works
//!
//! [data] is where the *CUBE* API response data are defined as [serde::de::Deserialize]-able types.
//! Most things in *CUBE* are linked to other things. For instance, from *plugins* a user can
//! create *plugin instances*.
//! [linked] defines wrappers which pair a response struct from [data] with a [reqwest::Client].
//! In the private submodules of `live`, associated methods are defined on specific wrapped objects.
//! For instance, [`linked::LinkedModel<AuthedPluginResponse>`] (type aliased as [ChrisPlugin]) has methods
//! for creating plugin instances.

mod data;
mod file;
mod linked;
mod live;

pub(crate) use data::*;
pub(crate) use file::*;
pub(crate) use linked::*;
pub use live::*;
