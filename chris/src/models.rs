//! Representations of data from *CUBE*.
//!
//! ## How It Works
//!
//! [data] is where the *CUBE* API response data are defined as [serde::de::Deserialize]-able types.
//! Most things in *CUBE* are linked to other things. For instance, from *plugins* a user can
//! create *plugin instances*.
//! [linked] defines wrappers which pair a response struct from [data] with a [reqwest::Client].
//! In the private submodules of `live`, associated methods are defined on specific wrapped objects.
//! For instance, [`linked::LinkedModel<PluginResponse>`] (type aliased as [Plugin]) has methods
//! for creating plugin instances.

pub mod data;
pub mod linked;

mod file;
pub(crate) mod live;

pub use file::*;
pub use live::*;
