//! Miscellaneous notes. (Where should I put this?)
//!
//! There is no consistent terminology used in the code. Though perhaps
//! some definitions are better than none:
//!
//! Made-up vocabulary:
//! - fname is the `fname` of an **existing** file in *CUBE*, e.g. `chris/feed_4/pl-dircopy_7/data/hello.txt`
//! - fname-like is a string which looks like some left-part of a fname. fname-like strings
//!   are appropriate values for the `path` argument of `api/v1/filebrowser/search/`.
//!   For instance, `chris/feed_4` is a fname-like (but not a valid fname).
use async_stream::stream;
use chris::errors::CUBEError;
use chris::models::{FeedId, FileResourceFname, PluginInstanceId};
use chris::ChrisClient;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    /// Substitutions for unallowed substrings for folder names.
    static ref FOLDER_SUBSTR_SUBSTITUTIONS: HashMap<&'static str, &'static str> = [
        ("/", "!SLASH!")
    ].into_iter().collect();
}

/// Wrapper around [Option<PathNamer>].
pub(crate) struct MaybeNamer {
    namer: Option<PathNamer>,
}

impl MaybeNamer {
    pub fn new(client: &ChrisClient, rename: bool) -> Self {
        let namer = if rename {
            Some(PathNamer::new(client.clone()))
        } else {
            None
        };
        Self { namer }
    }

    /// Calls the wrapped [PathNamer::rename] if Some,
    /// otherwise returns `fname` as a string.
    pub async fn rename(&mut self, fname: &FileResourceFname) -> String {
        if let Some(ref mut n) = self.namer {
            n.rename(fname).await
        } else {
            fname.to_string()
        }
    }

    /// Calls the wrapped [PathNamer::translate] if Some,
    /// otherwise returns `given_path` as a string.
    pub async fn translate(&mut self, given_path: &str) -> Result<String, TranslationError> {
        if let Some(ref mut n) = self.namer {
            n.translate(given_path).await
        } else {
            Ok(given_path.to_string())
        }
    }

    /// Calls the wrapped [PathNamer::try_get_feed_name] if Some,
    /// otherwise returns `folder` as a string.
    pub async fn try_get_feed_name(&mut self, folder: &str) -> String {
        if let Some(ref mut n) = self.namer {
            n.try_get_feed_name(folder).await
        } else {
            folder.to_string()
        }
    }

    /// Calls the wrapped [PathNamer::get_title_for] if Some,
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

impl Default for MaybeNamer {
    fn default() -> Self {
        MaybeNamer { namer: None }
    }
}

/// [PathNamer] is a struct which provides methods for renaming CUBE "swift" file paths
/// to human-readable file paths by replacing feed and plugin instance folder names with
/// feed names and plugin instance titles.
pub(crate) struct PathNamer {
    chris: ChrisClient,

    /// cache of plugin instance titles
    plinst_memo: HashMap<String, String>,

    /// cache of feed names
    feed_memo: HashMap<String, String>,

    /// When this [PathNamer] encounters an error from CUBE (except from 404 errors)
    /// `cube_error` is set to `true` so that it won't try to contact CUBE again.
    cube_error: bool,
}

impl PathNamer {
    fn new(chris: ChrisClient) -> Self {
        Self {
            chris,
            plinst_memo: Default::default(),
            feed_memo: Default::default(),
            cube_error: false,
        }
    }

    /// Tries to rename a path components of a feed file output's `fname` so that the folder names
    /// are changed to use the folder's corresponding feed name or plugin instance title.
    ///
    /// The renamed paths are more human-friendly for the purposes of downloading output folders.
    pub async fn rename(&mut self, fname: &FileResourceFname) -> String {
        let s: &str = fname.as_str(); // to help CLion with type hinting

        if let Some((username, feed_folder, feed_id, split)) = consume_feed_fname(s.split('/')) {
            let feed_name = self.get_feed_name(feed_id, feed_folder).await;
            let folders = self.rename_plugin_instances(split).await;
            if folders.is_empty() {
                format!("{}/{}", username, feed_name)
            } else {
                format!("{}/{}/{}", username, feed_name, folders)
            }
        } else {
            s.to_string()
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
    /// a given default value is returned, and [PathNamer::cube_error] is set to `true`.
    async fn get_feed_name(&mut self, id: FeedId, feed_folder: &str) -> String {
        if let Some(name) = self.feed_memo.get(feed_folder) {
            return name.to_string();
        }
        self.chris
            .get_feed(id)
            .await
            .map(|feed| feed.name)
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

    /// See [MaybeNamer::rename_plugin_instances]
    pub(crate) async fn rename_plugin_instances<'a, I>(&'a mut self, split: I) -> String
    where
        I: Iterator<Item = &'a str> + 'a,
    {
        self.stream_plugin_instance_folder_names(split)
            .collect::<Vec<String>>()
            .await
            .join("/")
    }

    fn stream_plugin_instance_folder_names<'a, I>(
        &'a mut self,
        mut split: I,
    ) -> impl Stream<Item = String> + '_
    where
        I: Iterator<Item = &'a str> + 'a,
    {
        stream! {
            // process up until "data"
            while let Some(folder) = split.next() {
                if folder == "data" {
                    yield folder.to_string();
                    break;
                }
                let title = self.get_title_for(folder).await;
                yield title;
            }

            // spit out the rest of the folders, which are
            // outputs created by the plugin instance
            while let Some(folder) = split.next() {
                yield folder.to_string();
            }
        }
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
            .map_err(PluginInstanceTitleError::CUBE)?;
        Ok(plinst.title)
    }

    /// Attempts to reverse operation of [Self:rename]. Untranslatable
    /// path components are left as-is.
    pub async fn translate(&mut self, path: &str) -> Result<String, TranslationError> {
        if let Some((username_folder, feed_name, joined_plinst_titles, data_folder, output_path)) =
            split_renamed_path(path)
        {
            let plinst_titles: Vec<&str> = joined_plinst_titles.split('/').collect();
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

    async fn plinst_titles2folders(
        &self,
        titles: &[&str],
    ) -> Result<Vec<String>, TranslationError> {
        Ok(titles.iter().map(|s| s.to_string()).collect())
    }

    /// Given the name of a feed, search CUBE for its ID *N* and return its folder name in the
    /// format "feed_*N*". If the feed is found, its name will be cached.
    ///
    /// If given a feed folder, do nothing and return it.
    pub async fn feed_name2folder(&self, feed_name: &str) -> Result<String, TranslationError> {
        if parse_feed_folder(feed_name).is_some() {
            return Ok(feed_name.to_string());
        }

        let query = &[("name", feed_name), ("limit", "1")];
        let search = self.chris.search_feeds(query);
        pin_mut!(search);
        search
            .try_next()
            .await
            .map_err(TranslationError::RequestError)?
            .map(|feed| format!("feed_{}", feed.id.0))
            .ok_or_else(|| TranslationError::FeedNotFound(feed_name.to_string()))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TranslationError {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),

    #[error("Cannot find feed with name \"{0}\"")]
    FeedNotFound(String),

    #[error("Cannot find plugin instance with title \"{0}\"")]
    PluginInstanceNotFound(String),
}

/// If given `path` looks like a fname-like of a feed output file which was renamed by
/// [PathNamer::rename], then split it into its components:
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
        .filter(|(root_folder, rest)| *root_folder != "SERVICES")
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
    for (from, to) in FOLDER_SUBSTR_SUBSTITUTIONS.iter() {
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
    CUBE(#[from] CUBEError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

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

    // TODO: use HTTP mocking to test PathNamer::rename
    // #[rstest]
    // #[tokio::test]
    // async fn test_try() -> anyhow::Result<()> {
    //     let account = CUBEAuth {
    //         username: Username::new("chris".to_string()),
    //         password: "chris1234".to_string(),
    //         url: CUBEApiUrl::try_from("https://cube.chrisproject.org/api/v1/")?,
    //         client: &reqwest::Client::new(),
    //     };
    //     let client = account.into_client().await?;
    //     let mut namer = PathNamer::new(client);
    //
    //     let example = FileResourceFname::from("chris/feed_1859/pl-dircopy_7933/pl-dcm2niix_7934/data/incoming_XR_Posteroanterior_(PA)_view_2021000000_3742127318.json");
    //     let actual = namer.rename(&example).await;
    //     dbg!(actual);
    //
    //     Ok(())
    // }
}
