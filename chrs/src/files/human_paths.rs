use chris::errors::CUBEError;
use chris::models::{FeedId, FileResourceFname, PluginInstanceId};
use chris::ChrisClient;
use std::collections::HashMap;
use async_stream::stream;
use futures::{Stream, StreamExt};
use itertools::Itertools;

/// [PathNamer] is a struct which provides memoization for the helper function
/// [PathNamer::rename].
pub(crate) struct PathNamer {
    chris: ChrisClient,

    /// cache of plugin instance titles
    memo: HashMap<String, String>,

    /// When this [PathNamer] encounters an error from CUBE (except from 404 errors)
    /// `cube_error` is set to `true` so that it won't try to contact CUBE again.
    cube_error: bool,
}

impl PathNamer {
    fn new(chris: ChrisClient) -> Self {
        Self {
            chris,
            memo: Default::default(),
            cube_error: false,
        }
    }

    /// Tries to rename a path components of a feed file output's `fname` so that the folder names
    /// are changed to use the folder's corresponding feed name or plugin instance title.
    ///
    /// The renamed paths are more human-friendly for the purposes of downloading output folders.
    pub async fn rename(&mut self, fname: &FileResourceFname) -> String {
        let s: &str = fname.as_str(); // to help CLion with type hinting

        if let Some((username, feed_id, split)) = consume_feed_fname(s.split("/")) {
            // TODO rename feed_id
            let feed_name = format!("feed_{}_renamed", *feed_id);
            let folders: Vec<String> = self.rename_plugin_instances(split).collect().await;
            let subpaths = folders.join("/");
            return format!("{}/{}/{}", username, feed_name, subpaths);
        } else {
            s.to_string()
        }
    }

    /// Consumes the given iterator. For every folder name which comes before the
    /// special value "data", try to ge the title for the plugin instance.
    /// If the plugin instance's title cannot be resolved, then a warning message
    /// is printed to stderr and the program continues.
    fn rename_plugin_instances<'a, I>(&'a mut self, mut split: I) -> impl Stream<Item = String> + '_
    where I: Iterator<Item = &'a str> + 'a {
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

    /// Retrieves plugin title from cache if available. Else, make a request to CUBE,
    /// cache the title, and then return it.
    ///
    /// In any case the plugin title cannot be found, a warning message is written
    /// to stderr explaining the problem.
    async fn get_title_for(&mut self, folder: &str) -> String {
        // first, check if previously cached
        if let Some(title) = self.memo.get(folder) {
            return title.clone();
        }

        // if previously encountered an error from CUBE, stop trying
        if self.cube_error {
            self.memo.insert(folder.to_string(), folder.to_string());
            return folder.to_string();
        }

        // else, try to parse and get from CUBE
        match self.get_from_cube(folder).await {
            Ok(title) => {
                self.memo.insert(title.clone(), title.clone());
                title
            }
            Err(e) => {
                eprintln!("WARNING: {:?}", e);
                self.cube_error = true;  // don't try to speak to CUBE again
                // default to using the folder name as-is
                self.memo.insert(folder.to_string(), folder.to_string());
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
}

/// Parse a folder name corresponding to a plugin instance's output files,
/// returning the plugin instance's id number.
fn parse_plinst_id(folder: &str) -> Result<PluginInstanceId, PluginInstanceTitleError> {
    folder
        .rsplit_once('_')
        .map(|(_name, sid)| sid.parse().ok())
        .flatten()
        .map(PluginInstanceId)
        .ok_or_else(|| PluginInstanceTitleError::Malformed(folder))
}

/// Consumes the first two items from the given iterator. If the second item is
/// recognized as a feed output folder, the consumed items, feed ID, and the
/// rest of the iterator is returned. Otherwise, the given iterator is dropped.
fn consume_feed_fname<'a, I>(mut iter: I) -> Option<(&'a str, FeedId, I)>
where
    I: Iterator<Item = &'a str>,
{
    if let Some(first) = iter.next() {
        iter.next()
            .map(parse_feed_folder)
            .flatten()
            .map(|feed_id| (first, feed_id, iter))
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
    use chris::auth::CUBEAuth;
    use chris::common_types::{CUBEApiUrl, Username};
    use rstest::*;

    #[rstest]
    fn test_consume_feed_fname() {
        let input = "chrisuser/feed_187/pl-fs-app_200/pl-ds-app_202/hello.json";
        let (username, feed_id, rest) = consume_feed_fname(input.split('/')).unwrap();
        assert_eq!(username, "chrisuser");
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
    #[tokio::test]
    async fn test_whatever() -> anyhow::Result<()> {
        let account = CUBEAuth {
            username: Username::new("chris".to_string()),
            password: "chris1234".to_string(),
            url: CUBEApiUrl::try_from("https://cube.chrisproject.org/api/v1/")?,
            client: &reqwest::Client::new(),
        };
        let client = account.into_client().await?;
        let mut namer = PathNamer::new(client);

        let example = FileResourceFname::from("chris/feed_1859/pl-dircopy_7933/pl-dcm2niix_7934/data/incoming_XR_Posteroanterior_(PA)_view_2021000000_3742127318.json");
        let actual = namer.rename(&example).await;
        dbg!(actual);

        Ok(())
    }
}
