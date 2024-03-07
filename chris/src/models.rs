//! Representations of data from *CUBE*.
//!
//! ## How It Works
//!
//! [data] is where the *CUBE* API response data are defined as [serde::de::Deserialize]-able types.
//! Most things in *CUBE* are linked to other things. For instance, from *plugins* a user can
//! create *plugin instances*.
//! [linked] defines wrappers which pair a response struct from [data] with a [reqwest::Client].
//! In the private submodules of `live`, associated methods are defined on specific wrapped objects.
//! For instance, [`LinkedModel<PluginResponse, crate::RwAccess>`] (type aliased as [Plugin])
//! has methods for creating plugin instances.

mod data;
mod file;
mod linked;
mod live;

pub use data::*;
pub use file::*;
pub use linked::*;
pub use live::*;
