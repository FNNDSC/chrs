//! Everything having to do with pagination of collection and search APIs from CUBE.

mod query;
#[allow(clippy::module_inception)]
mod search;
mod searches;

pub use query::QueryBuilder;
pub use search::*;
pub use searches::*;
