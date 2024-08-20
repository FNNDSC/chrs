//! Helper functions for dealing with understanding fnames.
//!
//! ## Notes
//!
//! There is no consistent terminology used in the code. Though perhaps
//! some definitions are better than none:
//!
//! Made-up vocabulary:
//!
//! - fname is the `fname` of an **existing** file in *CUBE*, e.g. `chris/feed_4/pl-dircopy_7/data/hello.txt`
//! - fname-like, a.k.a. _fnl_, is a string which looks like some left-part of a fname.
//!   fname-like strings are appropriate values for the `path` argument of
//!   `api/v1/filebrowser/search/`.
//!   For instance, `chris/feed_4` is a fname-like (but not a valid fname).
//!   _fnl_ is a superset of fname.

// FIXME remove disabled lint rules after done with everything
#![allow(dead_code)]
#![allow(unused_variables)]

use async_stream::stream;
use chris::errors::CubeError;
use chris::types::{CollectionUrl, CubeUrl, FeedId, PluginInstanceId};
use chris::{reqwest, RoClient};
use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use std::collections::HashMap;
use url::Url;

const FOLDER_SUBSTR_SUBSTITUTIONS: [(&str, &str); 1] = [("/", "!SLASH!")];

/// Wrapper around [`Option<ChrisPathHumanCoder>`].
#[derive(Default)]
pub struct MaybeChrisPathHumanCoder<'a> {
    namer: Option<ChrisPathHumanCoder<'a>>,
}

impl<'a> MaybeChrisPathHumanCoder<'a> {
    pub fn new(client: &'a RoClient, rename: bool) -> Self {
        let namer = if rename {
            Some(ChrisPathHumanCoder::new(client))
        } else {
            None
        };
        Self { namer }
    }
}

impl MaybeChrisPathHumanCoder<'_> {
    /// Calls the wrapped [ChrisPathHumanCoder::decode] if Some,
    /// otherwise returns `fname` as a string.
    pub async fn decode(&mut self, fname: impl AsRef<str>) -> String {
        if let Some(ref mut n) = self.namer {
            n.decode(fname).await
        } else {
            fname.as_ref().to_string()
        }
    }

    // BLOCKED by https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530
    // Here we want to use the same code for logged in users vs anonymous users,
    // however since anonymous users can't use the same plugins/instances/search/
    // endpoint as logged in users, implementation would be messy.
    // pub fn canonicalize(&mut self, _ufn: UnionFnameLike) -> FnameLike {
    //     unimplemented!()
    // }

    /// Calls the wrapped [ChrisPathHumanCoder::encode] if Some,
    /// otherwise returns `given_path` as a string.
    pub async fn translate(&mut self, given_path: &str) -> Result<String, TranslationError> {
        if let Some(ref mut n) = self.namer {
            n.encode(given_path).await
        } else {
            Ok(given_path.to_string())
        }
    }

    /// Calls the wrapped [ChrisPathHumanCoder::try_get_feed_name] if Some,
    /// otherwise returns `folder` as a string.
    pub async fn try_get_feed_name(&mut self, folder: &str) -> String {
        if let Some(ref mut n) = self.namer {
            n.try_get_feed_name(folder).await
        } else {
            folder.to_string()
        }
    }

    /// Calls the wrapped [ChrisPathHumanCoder::get_title_for] if Some,
    /// otherwise returns `folder` as a string.
    pub async fn get_title_for(&mut self, folder: &str) -> String {
        if let Some(ref mut n) = self.namer {
            n.get_title_for(folder).await
        } else {
            folder.to_string()
        }
    }

    /// Consumes the given iterator. For every folder name which comes before the
    /// special value "data", try to ge the title for the plugin instance.
    /// If the plugin instance's title cannot be resolved, then a warning message
    /// is printed to stderr and the program continues.
    pub(crate) async fn rename_plugin_instances<'a, I>(&'a mut self, split: I) -> String
    where
        I: Iterator<Item = &'a str> + 'a,
    {
        if let Some(ref mut n) = self.namer {
            n.rename_plugin_instances(split).await
        } else {
            split.collect::<Vec<&str>>().join("/")
        }
    }
}

/// [ChrisPathHumanCoder] provides methods for renaming CUBE file paths to more-easily
/// human-readable file paths by replacing feed and plugin instance folder names with
/// feed names and plugin instance titles respectively.
pub(crate) struct ChrisPathHumanCoder<'a> {
    chris: &'a RoClient,

    /// cache of plugin instance titles
    plinst_memo: HashMap<String, String>,

    /// cache of feed names
    feed_memo: HashMap<String, String>,

    /// When this [ChrisPathHumanCoder] encounters an error from CUBE (except from 404 errors)
    /// `cube_error` is set to `true` so that it won't try to contact CUBE again.
    cube_error: bool,
}

impl<'a> ChrisPathHumanCoder<'a> {
    fn new(chris: &'a RoClient) -> Self {
        Self {
            chris,
            plinst_memo: Default::default(),
            feed_memo: Default::default(),
            cube_error: false,
        }
    }
}

impl ChrisPathHumanCoder<'_> {
    /// Tries to rename a path components of a feed file output's `fname` so that the folder names
    /// are changed to use the folder's corresponding feed name or plugin instance title.
    ///
    /// The renamed paths are more human-friendly for the purposes of downloading output folders.
    pub async fn decode(&mut self, fname: impl AsRef<str>) -> String {
        if let Some((username, feed_folder, feed_id, split)) =
            consume_feed_fname(fname.as_ref().split('/'))
        {
            let feed_name = self.get_feed_name(feed_id, feed_folder).await;
            let folders = self.rename_plugin_instances(split).await;
            if folders.is_empty() {
                format!("{}/{}", username, feed_name)
            } else {
                format!("{}/{}/{}", username, feed_name, folders)
            }
        } else {
            fname.as_ref().to_string()
        }
    }

    /// If a feed ID can be parsed from the given folder name, try and
    /// get its name from CUBE. In any case that is not possible, the folder
    /// name is simply returned as a string.
    pub async fn try_get_feed_name(&mut self, folder: &str) -> String {
        if let Some(id) = parse_feed_folder(folder) {
            self.get_feed_name(id, folder).await
        } else {
            folder.to_string()
        }
    }

    /// Gets (and caches) the feed name for the specified feed ID. If unable to, then
    /// a given default value is returned, and [ChrisPathHumanCoder::cube_error] is set to `true`.
    async fn get_feed_name(&mut self, id: FeedId, feed_folder: &str) -> String {
        if let Some(name) = self.feed_memo.get(feed_folder) {
            return name.to_string();
        }
        self.chris
            .get_feed(id)
            .await
            .map(|feed| feed.object.name)
            .map(substitute_unallowed)
            .map(|name| this_or_that(name, feed_folder))
            .map(|f| self.cache_feed_name(feed_folder, f))
            .unwrap_or_else(|e| {
                eprintln!(
                    "WARNING: could not get feed name for \"{}\". {:?}",
                    feed_folder, e
                );
                self.cube_error = true;
                feed_folder.to_string()
            })
    }

    fn cache_feed_name(&mut self, folder: &str, feed_name: String) -> String {
        self.feed_memo
            .insert(folder.to_string(), feed_name.to_string());
        feed_name
    }

    /// See [MaybeChrisPathHumanCoder::rename_plugin_instances]
    pub(crate) async fn rename_plugin_instances<'b, I>(&'b mut self, mut split: I) -> String
    where
        I: Iterator<Item = &'b str> + 'b,
    {
        let plugin_instance_folder_names = stream! {
            // process up until "data"
            for folder in split.by_ref() {
                if folder == "data" {
                    yield folder.to_string();
                    break;
                }
                if folder.is_empty() {  // trailing slash causes last element to be empty
                    break;
                }
                let title = self.get_title_for(folder).await;
                yield title;
            }

            // spit out the rest of the folders, which are
            // outputs created by the plugin instance
            for folder in split.by_ref() {
                yield folder.to_string()
            }
        };
        plugin_instance_folder_names
            .collect::<Vec<String>>()
            .await
            .join("/")
    }

    /// Retrieves plugin instance title from cache if available. Else, make a request to CUBE,
    /// cache the title, and then return it.
    ///
    /// In any case the plugin title cannot be found, a warning message is written
    /// to stderr explaining the problem. This warning is only shown once.
    pub async fn get_title_for(&mut self, folder: &str) -> String {
        // first, check if previously cached
        if let Some(title) = self.plinst_memo.get(folder) {
            return title.clone();
        }

        // if previously encountered an error from CUBE, stop trying
        if self.cube_error {
            self.plinst_memo
                .insert(folder.to_string(), folder.to_string());
            return folder.to_string();
        }

        // else, try to parse and get from CUBE
        match self.get_from_cube(folder).await {
            Ok(title) => {
                let title = substitute_unallowed(title);
                let title = this_or_that(title, folder);
                self.plinst_memo.insert(folder.to_string(), title.clone());
                title
            }
            Err(e) => {
                eprintln!("WARNING: {:?}", e);
                self.cube_error = true; // don't try to speak to CUBE again
                                        // default to using the folder name as-is
                self.plinst_memo
                    .insert(folder.to_string(), folder.to_string());
                folder.to_string()
            }
        }
    }

    /// Get from CUBE the title of the plugin instance which corresponds to the given folder name.
    async fn get_from_cube<'a>(
        &'a self,
        folder: &'a str,
    ) -> Result<String, PluginInstanceTitleError> {
        let id = parse_plinst_id(folder)?;
        let plinst = self
            .chris
            .get_plugin_instance(id)
            .await
            .map_err(PluginInstanceTitleError::Cube)?;
        Ok(plinst.object.title)
    }

    /// Attempts to reverse operation of [Self::decode]. Untranslatable
    /// path components are left as-is.
    pub async fn encode(&mut self, path: &str) -> Result<String, TranslationError> {
        if let Some((username_folder, feed_name, joined_plinst_titles, data_folder, output_path)) =
            split_renamed_path(path)
        {
            let plinst_titles: Vec<&str> = joined_plinst_titles
                .split('/')
                .filter(|s| !s.is_empty())
                .collect();
            let (feed_folder, plinst_folders) = tokio::try_join!(
                self.feed_name2folder(feed_name),
                self.plinst_titles2folders(&plinst_titles)
            )?;

            // cache the stuff
            self.feed_memo
                .insert(feed_folder.to_string(), feed_name.to_string());
            for (plinst_folder, plinst_title) in plinst_folders.iter().zip(plinst_titles.iter()) {
                self.plinst_memo
                    .insert(plinst_folder.to_string(), plinst_title.to_string());
            }

            let joined_plinst_folders = plinst_folders.join("/");
            let components = [
                username_folder,
                &feed_folder,
                &joined_plinst_folders,
                data_folder,
                output_path,
            ];
            let fname = components.into_iter().filter(|c| !c.is_empty()).join("/");
            Ok(fname)
        } else {
            Ok(path.to_string())
        }
    }

    /// Convert plugin instance titles to plugin instance folder names.
    async fn plinst_titles2folders(
        &self,
        titles: &[&str],
    ) -> Result<Vec<String>, TranslationError> {
        // Future work:
        // The searches can be made more strict using information about `previous_id`
        // and `feed_id`, which we know from the full given path.
        futures::stream::iter(titles)
            .map(|title| self.plinst_title2folder(title))
            .buffered(10)
            .try_collect()
            .await
    }

    async fn plinst_title2folder(&self, title: &str) -> Result<String, TranslationError> {
        Err(TranslationError::PluginInstanceNotFound(
            "Not implemented".to_string(),
        ))
        // let query = &[("title", title), ("limit", "1")];
        // let search = self.chris.search_plugin_instances(query);
        // pin_mut!(search);
        // let folder = search
        //     .try_next()
        //     .await
        //     .map_err(TranslationError::RequestError)?
        //     .map(|plinst| format!("{}_{}", plinst.plugin_name.as_str(), plinst.id.0))
        //     .ok_or_else(|| TranslationError::PluginInstanceNotFound(title.to_string()))
        //     .or_else(|e| {
        //         // given value is not a plugin instance ID, but looks like a valid folder already
        //         parse_plinst_id(title)
        //             .map_err(|_| e)
        //             .map(|_| title.to_string())
        //     });
        // if let Some(second) = search.try_next().await? {
        //     Err(TranslationError::AmbiguousPluginInstanceTitleError(
        //         second.title,
        //     ))
        // } else {
        //     folder
        // }
    }

    /// Given the name of a feed, search CUBE for its ID *N* and return its folder name in the
    /// format "feed_*N*". If the feed is found, its name will be cached.
    async fn feed_name2folder(&self, feed_name: &str) -> Result<String, TranslationError> {
        Err(TranslationError::FeedNotFound(
            "not implemented".to_string(),
        ))
        // let query = &[("name", feed_name), ("limit", "1")];
        // let search = self.chris.search_feeds(query);
        // pin_mut!(search);
        // let folder = search
        //     .try_next()
        //     .await
        //     .map_err(TranslationError::RequestError)?
        //     .map(|feed| format!("feed_{}", feed.id.0))
        //     .ok_or_else(|| TranslationError::FeedNotFound(feed_name.to_string()))
        //     .or_else(|e| {
        //         // given value is not a feed name, but looks like a valid feed folder already
        //         parse_feed_folder(feed_name)
        //             .map(|_| feed_name.to_string())
        //             .ok_or(e)
        //     });
        // if let Some(second) = search.try_next().await? {
        //     Err(TranslationError::AmbiguousFeedNameError(second.name))
        // } else {
        //     folder
        // }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TranslationError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),

    #[error("Ambiguous name: \"{0}\" (must give canonical fname with numerical ID)")]
    // TODO show matching feed IDs
    AmbiguousFeedNameError(String),

    #[error("Ambiguous title: \"{0}\" (must give canonical fname with numerical ID)")]
    // TODO show matching plugin instance IDs
    AmbiguousPluginInstanceTitleError(String),

    #[error("Cannot find feed with name \"{0}\"")]
    FeedNotFound(String),

    #[error("Cannot find plugin instance with title \"{0}\"")]
    PluginInstanceNotFound(String),
}

/// Figure out whether the input is a URL or a path.
/// If it's a path, then construct a search URL from it.
///
/// Returns the URL and the length of the given fname, or 0
/// if not given a fname.
pub fn parse_src(src: &str, address: &CubeUrl) -> CollectionUrl {
    if src.starts_with(address.as_str()) {
        return src.into();
    }
    if src.starts_with("SERVICES") {
        if src.starts_with("SERVICES/PACS") {
            return to_search(address, "pacsfiles", src);
        }
        return to_search(address, "servicefiles", src);
    }
    if let Some((_username, subdir)) = src.split_once('/') {
        if subdir.starts_with("uploads") {
            return to_search(address, "uploadedfiles", src);
        }
    }
    to_search(address, "files", src)
}

/// Create a search API URL for the endpoint and fname.
fn to_search(address: &CubeUrl, endpoint: &str, fname: &str) -> CollectionUrl {
    Url::parse_with_params(
        &format!("{}{}/search/", address, endpoint),
        &[("fname", fname)],
    )
    .unwrap()
    .as_str()
    .into()
}

/// If given `path` looks like a fname-like of a feed output file which was renamed by
/// [ChrisPathHumanCoder::decode], then split it into its components:
///
/// 1. username, e.g. "chris"
/// 2. feed name
/// 3. plugin instance titles separated by slashes,
///    e.g. "First Plugin Instance/Second Plugin Instance"
/// 4. data folder (if present), either "data" or ""
/// 5. output path (if present), e.g. "arbitrary/data/path/filename.json"
///
/// If given path is _not_ a feed output fname-like, for instance, a PACS fname-like or
/// uploaded file fname, then `None` is returned.
fn split_renamed_path(path: &str) -> Option<(&str, &str, &str, &str, &str)> {
    path.split_once('/')
        // all top-level folders besides "SERVICES" contain user files
        .filter(|(root_folder, _rest)| *root_folder != "SERVICES")
        .map(|(root_folder, rest)| {
            rest.split_once('/')
                .map(|(feed_folder, rest)| (root_folder, feed_folder, rest))
                .unwrap_or((root_folder, rest, ""))
        })
        // all subdirs of a user's directory besides "uploads" are feed output folders
        .filter(|(_, feed_folder, _)| *feed_folder != "uploads")
        .map(|(root_folder, feed_folder, rest)| {
            let (plinst_folders, data_folder, output_path) = rest
                .split_once("/data")
                .map(|(plinst_folders, data_path)| (plinst_folders, "data", data_path))
                .unwrap_or((rest, "", ""));
            (
                root_folder,
                feed_folder,
                plinst_folders.trim_end_matches('/'),
                data_folder,
                output_path.trim_start_matches('/'),
            )
        })
}

fn substitute_unallowed(mut folder_name: String) -> String {
    for (from, to) in FOLDER_SUBSTR_SUBSTITUTIONS {
        folder_name = folder_name.replace(from, to)
    }
    folder_name
}

fn this_or_that(a: String, b: &str) -> String {
    if a.is_empty() {
        b.to_string()
    } else {
        a
    }
}

/// Parse a folder name corresponding to a plugin instance's output files,
/// returning the plugin instance's id number.
fn parse_plinst_id(folder: &str) -> Result<PluginInstanceId, PluginInstanceTitleError> {
    folder
        .rsplit_once('_')
        .and_then(|(_name, sid)| sid.parse().ok())
        .map(PluginInstanceId)
        .ok_or(PluginInstanceTitleError::Malformed(folder))
}

/// Consumes the first two items from the given iterator. If the second item is
/// recognized as a feed output folder, the consumed items, feed ID, and the
/// rest of the iterator is returned. Otherwise, the given iterator gets dropped.
fn consume_feed_fname<'a, I>(mut iter: I) -> Option<(&'a str, &'a str, FeedId, I)>
where
    I: Iterator<Item = &'a str>,
{
    if let Some(first) = iter.next() {
        iter.next()
            .and_then(|f| parse_feed_folder(f).map(|n| (f, n)))
            .map(|(feed_folder, feed_id)| (first, feed_folder, feed_id, iter))
    } else {
        None
    }
}

/// Parse a feed ID number from a folder which corresponds to a feed's output files.
fn parse_feed_folder(folder: &str) -> Option<FeedId> {
    if let Some((prefix, num)) = folder.split_once('_') {
        if prefix == "feed" {
            num.parse().map(FeedId).ok()
        } else {
            None
        }
    } else {
        None
    }
}

/// Possible errors which might happen when trying to get the title of a plugin instance
/// given a folder name containing the plugin instance's files.
#[derive(thiserror::Error, Debug)]
enum PluginInstanceTitleError<'a> {
    #[error("malformed plugin instance folder \"{0}\"")]
    Malformed(&'a str),

    #[error(transparent)]
    Cube(#[from] CubeError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chris::types::FileResourceFname;
    use chris::{AnonChrisClient, EitherClient};
    use rstest::*;

    #[rstest]
    #[case(
        "https://example.com/api/v1/uploadedfiles/search/?fname_icontains=gluten",
        "https://example.com/api/v1/uploadedfiles/search/?fname_icontains=gluten"
    )]
    #[case(
        "SERVICES/PACS/orthanc",
        "https://example.com/api/v1/pacsfiles/search/?fname=SERVICES%2FPACS%2Forthanc"
    )]
    #[case(
        "waffle/uploads/powdered_sugar",
        "https://example.com/api/v1/uploadedfiles/search/?fname=waffle%2Fuploads%2Fpowdered_sugar"
    )]
    #[case(
        "cereal/feed_1/pl-dircopy_1",
        "https://example.com/api/v1/files/search/?fname=cereal%2Ffeed_1%2Fpl-dircopy_1"
    )]
    fn test_parse_src_url(
        #[case] src: &str,
        #[case] expected: &'static str,
        example_address: &CubeUrl,
    ) {
        assert_eq!(
            parse_src(src, example_address),
            CollectionUrl::from_static(expected)
        );
    }

    #[rstest]
    fn test_consume_feed_fname() {
        let input = "chrisuser/feed_187/pl-fs-app_200/pl-ds-app_202/hello.json";
        let (username, feed_folder, feed_id, rest) = consume_feed_fname(input.split('/')).unwrap();
        assert_eq!(username, "chrisuser");
        assert_eq!(feed_folder, "feed_187");
        assert_eq!(feed_id, FeedId(187));
        assert_eq!(
            rest.collect::<Vec<&str>>(),
            vec!["pl-fs-app_200", "pl-ds-app_202", "hello.json"]
        );
    }

    #[rstest]
    #[case("pl-dircopy_3", Some(3))]
    #[case("pl-simpledsapp_45", Some(45))]
    #[case("noprefix_789", Some(789))]
    #[case("what", None)]
    fn test_parse_plinst_id(#[case] folder: &str, #[case] expected: Option<u32>) {
        assert_eq!(parse_plinst_id(folder).ok(), expected.map(PluginInstanceId))
    }

    #[rstest]
    #[case("feed_1", Some(1))]
    #[case("feed_11", Some(11))]
    #[case("feed_123", Some(123))]
    #[case("food_123", None)]
    #[case("what", None)]
    fn test_parse_feed_folder(#[case] folder: &str, #[case] expected: Option<u32>) {
        assert_eq!(parse_feed_folder(folder), expected.map(FeedId))
    }

    #[rstest]
    #[case("i am ok!", "i am ok!")]
    #[case("i am not ok/bad...", "i am not ok!SLASH!bad...")]
    fn test_replace_unallowed(#[case] folder: &str, #[case] expected: &str) {
        assert_eq!(
            substitute_unallowed(folder.to_string()),
            expected.to_string()
        )
    }

    #[rstest]
    #[case("", None)]
    #[case("SERVICES/PACS/something...", None)]
    #[case("chris/uploads", None)]
    #[case("chris/uploads/something", None)]
    #[case("christopher/feed_12", Some(("christopher", "feed_12", "", "", "")))]
    #[case("chris/feed_12/pl-dircopy_17/pl-something_18/data/subfolder/file.json", Some(("chris", "feed_12", "pl-dircopy_17/pl-something_18", "data", "subfolder/file.json")))]
    #[case("chris/Feed Name/pl-dircopy_17/Plinst Title", Some(("chris", "Feed Name", "pl-dircopy_17/Plinst Title", "", "")))]
    #[case("chris/Feed Name/pl-dircopy_17/Plinst Title/", Some(("chris", "Feed Name", "pl-dircopy_17/Plinst Title", "", "")))]
    #[case("chris/Feed Name/pl-dircopy_17/Plinst Title/data", Some(("chris", "Feed Name", "pl-dircopy_17/Plinst Title", "data", "")))]
    #[case("chris/Feed Name/pl-dircopy_17/Plinst Title/data/", Some(("chris", "Feed Name", "pl-dircopy_17/Plinst Title", "data", "")))]
    #[case("chris/Feed Name/pl-dircopy_17/Plinst Title/data/subfolder/file.json", Some(("chris", "Feed Name", "pl-dircopy_17/Plinst Title", "data", "subfolder/file.json")))]
    fn test_split_renamed_path(
        #[case] path: &str,
        #[case] expected: Option<(&str, &str, &str, &str, &str)>,
    ) {
        let actual = split_renamed_path(path);
        assert_eq!(actual, expected);
    }

    #[fixture]
    #[once]
    fn example_address() -> CubeUrl {
        CubeUrl::try_from("https://example.com/api/v1/").unwrap()
    }

    #[rstest]
    #[tokio::test]
    async fn test_try() {
        let cube_url = CubeUrl::from_static(
            "https://cube-for-testing-chrisui.apps.shift.nerc.mghpcc.org/api/v1/",
        );
        let anon_client = AnonChrisClient::build(cube_url)
            .unwrap()
            .connect()
            .await
            .unwrap();
        let client = EitherClient::Anon(anon_client).into_ro();
        let mut namer = ChrisPathHumanCoder::new(&client);

        let example = FileResourceFname::from("chrisui/feed_310/pl-dircopy_313/pl-unstack-folders_314/pl-mri-preview_875/data/fetalmri-template-22.txt");
        let expected = "chrisui/Brain Volume Data/pl-dircopy_313/pl-unstack-folders_314/Measure volume and create center-slice figures/data/fetalmri-template-22.txt";
        let actual = namer.decode(&example).await;
        assert_eq!(&actual, expected)
    }
}
