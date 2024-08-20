//! Client library for [_ChRIS_](https://chrisproject.org/) built
//! on [reqwest](https://crates.io/crates/reqwest).
//!
//! ## Introduction
//!
//! _ChRIS_ is a platform for scientific and medical compute. In _ChRIS_,
//! objects known as _ChRIS plugins_ represent data processing software.
//! A _user_ can run a _plugin_ by creating a _plugin instance._ Chains
//! of _plugin instances_ are organized as _feeds._
//!
//! Most of the _ChRIS_ API, also known as _CUBE_, requires a user account.
//! A subset of API endpoints can be accessed anonymously in read-only mode.
//! For example, anyone can list the plugins of a _CUBE_. However, a user
//! account is required to create _plugin instances_ and browse non-public _feeds_.
//!
//! Pro-tip: read the integration test code for code examples.
//!
//! - <https://github.com/FNNDSC/chrs/blob/master/chris/tests/test_public.rs>
//! - <https://github.com/FNNDSC/chrs/blob/master/chris/tests/test_logged_in.rs>
//!
//! ### Authentication
//!
//! Typically, you start off with a _CUBE_ URL, username, and password. First,
//! obtain a token with [`Account::get_token`], then call [`ChrisClient::build`].
//!
//! For anonymous access, simply call [`AnonChrisClient::build`].
//!
//! ### Access Modes
//!
//! All client structs are generic over [Access], being either [RwAccess]
//! or [RoAccess]. In many cases, methods are only available to an object
//! when it is of type [RwAccess]. e.g. [`PluginRw::create_instance`] is
//! defined for the type [`Plugin<RwAccess>`], however no such method exists
//! for [`Plugin<RoAccess>`].
//!
//! In situations where either [RwAccess] or [RoAccess] works, implement
//! your function to be generic over [Access]. For example,
//!
//! ```
//! use chris::{Access, BaseChrisClient, Plugin};
//!
//! async fn generic_example<A: Access, C: BaseChrisClient<A>>(chris: &C) -> Plugin<A> {
//!     todo!()
//! }
//! ```
//!
//! Types which are generic with [RoAccess] typically have a subset of the methods of
//! the same type but generic with [RwAccess]. If [RoAccess] is good enough for you,
//! consider [EitherClient]:
//!
//! ```
//! use chris::{BaseChrisClient, EitherClient, PluginRo};
//!
//! async fn either_client_example(chris: &EitherClient) -> PluginRo {
//!     chris.plugin().name("pl-dircopy").search().get_only().await.unwrap()
//! }
//! ```
//!
//! It can be convenient to convert objects from `T<Access>` to `T<RoAccess>`
//! by calling [AuthedChrisClient::into_ro], [EitherClient::into_ro],
//! [search::Search::into_ro], [search::QueryBuilder::into_ro], or [LinkedModel::from].
//!
//! ### Searching Collections
//!
//! The _ChRIS_ API is designed around the idea of objects and collections.
//! For example, the plugin collection API is `api/v1/plugins/`, while specific
//! plugins are retrieved from `api/v1/plugins/1/`, `api/v1/plugins/2/`, and so on.
//! Collections usually also have a search API, e.g. `api/v1/plugins/search/?name=pl-example`.
//!
//! Many functions return a [search::Search]. For example, suppose you want a [Plugin].
//! You need to search for it like this:
//!
//! ```
//! use chris::{Access. BaseChrisClient, Plugin};
//!
//! async fn get_plugin_example<A: Access, C: BaseChrisClient<A>>(chris: &C) -> Plugin<A> {
//!     chris
//!         .plugin()
//!         .name("pl-example")
//!         .search()
//!         .get_first()
//!         .await
//!         .unwrap()
//!         .expect("plugin not found")
//! }
//! ```
//!
//! You can either use [`search::Search::get_first`] if you want [`Option<T>`], or
//! [`search::Search::get_only()`] to get just `T`.
//!
//! ### Response Data, Linked v.s. Unlinked Structs
//!
//! Structs which represent JSON API response data as-is are defined in
//! `src/models/data.rs` and follow the naming convention `*Response`, e.g. [PluginResponse].
//! These response data structs, in combination with an [Access] and a [request::Client, are
//! the generic parameters of [LinkedModel]. Specific methods are defined for generic type
//! combinations. For example, you can get the note of a feed when you have RW access. Hence,
//! the method [`FeedRw::note`] is defined for [`LinkedModel<FeedResponse, RwAccess>`].
//!
//! In some situations, the API response data contains only a link to the object (e.g.
//! `"https://example.org/api/v1/5/"` instead of the full object (e.g.
//! `{"id": 5, "name": "Example feed", "url": "https://example.org/api/v1/5/", ...}`).
//! It would be costly to make an additional HTTP GET to create the full [LinkedModel].
//! Instead, a [LazyLinkedModel] is returned, which works the same as a [LinkedModel] but
//! is missing the actual object data. The object data can be obtained by calling
//! [LazyLinkedModel::get].

mod client;
mod models;
mod requests;

// pub mod auth;
pub mod errors;
// pub mod pipeline;
mod account;
pub mod search;
pub mod types;

pub use account::Account;
pub use client::access::{Access, RoAccess, RwAccess};
pub use client::anon::AnonChrisClient;
pub use client::authed::{ChrisClient, AuthedChrisClient};
pub use client::base::BaseChrisClient;
pub use client::either::{EitherClient, RoClient};
pub use client::filebrowser::{FileBrowser, FileBrowserEntry};
pub use models::*;

// re-export
pub use reqwest;
