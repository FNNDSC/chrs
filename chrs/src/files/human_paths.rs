use async_stream::stream;
use chris::errors::CUBEError;
use chris::models::{FileResourceFname, PluginInstanceId};
use chris::ChrisClient;
use std::collections::HashMap;

pub(crate) struct PathNamer {
    chris: ChrisClient,

    /// cache of plugin instance titles
    memo: HashMap<PluginInstanceId, String>,
}

impl PathNamer {
    fn new(chris: ChrisClient) -> Self {
        Self {
            chris,
            memo: Default::default(),
        }
    }

    pub async fn rename(&mut self, fname: &FileResourceFname) -> anyhow::Result<String> {
        let s: &str = fname.as_str(); // to help CLion with type hinting
        let mut iter = s.split("/");

        // TODO combine these if statements into one line
        // first part, if exists, is a username
        if let Some(first) = iter.next() {
            if let Some(second) = iter.next() {
                if second.starts_with("feed_") {
                    return self.rename_plugin_instances(iter).await;
                }
            }
        }
        Ok(s.to_string())
    }

    async fn rename_plugin_instances(
        &mut self,
        mut split: impl Iterator<Item = &str>,
    ) -> anyhow::Result<String> {
        // TODO returns a stream

        while let Some(folder) = split.next() {
            if folder == "data" {
                dbg!(folder);
                break;
            }
            if let Some((_name, sid)) = folder.rsplit_once('_') {
                let id = PluginInstanceId { 0: sid.parse()? };
                let title = self.get_title_for(id).await?.ok_or_else(|| {
                    let m = format!("Could not find title for plugin instance \"{}\"", folder);
                    anyhow::Error::msg(m)
                })?;
                dbg!(title);
            }
        }

        Ok("".to_string())
    }

    /// Retrieves plugin title from cache if available. Else, make a request to CUBE,
    /// cache the title, and then return it.
    async fn get_title_for(&mut self, id: PluginInstanceId) -> Result<Option<String>, CUBEError> {
        if let Some(title) = self.memo.get(&id) {
            return Ok(Some(title.clone()));
        }
        dbg!(self.chris.get_plugin_instance(id).await);
        if let Some(plinst) = self.chris.get_plugin_instance(id).await? {
            dbg!(&plinst);
            self.memo.insert(plinst.id, plinst.title.clone());
            return Ok(Some(plinst.title));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chris::auth::CUBEAuth;
    use chris::common_types::{CUBEApiUrl, Username};
    use chris::ChrisClient;
    use rstest::*;

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
        let actual = namer.rename(&example).await?;
        dbg!(actual);

        Ok(())
    }
}
