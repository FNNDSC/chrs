use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre;
use color_eyre::eyre::{bail, OptionExt, Result};
use futures::TryStreamExt;
use itertools::Itertools;
use std::fmt::Display;

use chris::types::PluginInstanceId;
use chris::{
    Access, BaseChrisClient, ChrisClient, EitherClient, LinkedModel, PluginInstance,
    PluginInstanceResponse, PluginInstanceRo, PluginInstanceRw,
};

/// A user-provided string which is supposed to refer to an existing plugin instance
/// or _ChRIS_ file path.
///
/// ## Limitations
///
/// A valid absolute path like `rudolph` (which is just his username) will be misidentified as
/// [GivenPluginInstance::Title] instead of [GivenPluginInstance::AbsolutePath].
#[derive(Debug, PartialEq, Clone)]
pub enum GivenPluginInstance {
    Title(String),
    Id(PluginInstanceId, String),
    RelativePath(String),
    AbsolutePath(String),
}

impl Default for GivenPluginInstance {
    fn default() -> Self {
        Self::RelativePath(".".to_string())
    }
}

impl Display for GivenPluginInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_arg_str())
    }
}

impl From<String> for GivenPluginInstance {
    fn from(value: String) -> Self {
        if value.is_empty() {
            return Default::default();
        }
        if let Some((left, right)) = value.split_once('/') {
            if left == "pi" || left == "plugininstance" {
                return parse_as_id_or_title(right, &value);
            }
        }
        if let Some(id) = parse_id_from_url(&value) {
            return GivenPluginInstance::Id(id, value);
        }
        if starts_with_dots(&value) {
            return GivenPluginInstance::RelativePath(value);
        }
        if looks_like_well_known_absolute_path(&value) {
            return GivenPluginInstance::AbsolutePath(value);
        }
        parse_as_id_or_title(&value, &value)
    }
}

fn parse_as_id_or_title(value: &str, original: &str) -> GivenPluginInstance {
    value
        .parse::<u32>()
        .map(PluginInstanceId)
        .map(|id| GivenPluginInstance::Id(id, original.to_string()))
        .unwrap_or_else(|_e| GivenPluginInstance::Title(value.to_string()))
}

fn starts_with_dots(value: &str) -> bool {
    value == "."
        || ["./", "..", "../"]
            .into_iter()
            .any(|s| value.starts_with(s))
}

fn looks_like_well_known_absolute_path(value: &str) -> bool {
    value == "SERVICES"
        || value.starts_with("SERVICES/PACS")
        || value == "PIPELINES"
        || value.starts_with("PIPELINES/")
        || looks_like_feed_output_path(value)
}

fn looks_like_feed_output_path(value: &str) -> bool {
    value
        .split_once('/')
        .map(|(_, r)| r)
        .map(|folder| folder.split_once('/').map(|(l, _)| l).unwrap_or(folder))
        .and_then(|folder| folder.strip_prefix("feed_"))
        .map(|feed_id| feed_id.parse::<u32>().is_ok())
        .unwrap_or(false)
}

fn parse_id_from_url(url: &str) -> Option<PluginInstanceId> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return None;
    }
    url.split_once("/api/v1/plugins/instances/")
        .map(|(_, right)| right)
        .and_then(|s| s.strip_suffix('/'))
        .and_then(|s| s.parse().ok())
        .map(PluginInstanceId)
}

impl GivenPluginInstance {
    /// Get the value. In the case where the value was originally a plugin instance URL,
    /// the URL is returned (not the parsed ID).
    pub fn as_arg_str(&self) -> &str {
        match self {
            GivenPluginInstance::Title(title) => title,
            GivenPluginInstance::Id(_, original) => original,
            GivenPluginInstance::RelativePath(path) => path,
            GivenPluginInstance::AbsolutePath(path) => path,
        }
    }

    pub async fn get_using_either(
        self,
        client: &EitherClient,
        old: Option<PluginInstanceId>,
    ) -> Result<PluginInstanceRo> {
        match self {
            Self::Id(id, _) => client
                .get_plugin_instance(id)
                .await
                .map_err(eyre::Error::new),
            Self::Title(title) => get_by_title_ro(client, title, old).await,
            Self::RelativePath(path) => match client {
                EitherClient::Anon(c) => get_relative_path_as_plinst(c, old, path).await,
                EitherClient::LoggedIn(c) => get_relative_path_as_plinst(c, old, path)
                    .await
                    .map(|p| p.into()),
            },
            Self::AbsolutePath(path) => match client {
                EitherClient::Anon(c) => get_plinst_of_path(c, &path).await,
                EitherClient::LoggedIn(c) => get_plinst_of_path(c, &path).await.map(|p| p.into()),
            },
        }
    }

    pub async fn get_using_rw(
        self,
        client: &ChrisClient,
        old: Option<PluginInstanceId>,
    ) -> Result<PluginInstanceRw> {
        match self {
            GivenPluginInstance::Title(title) => search_title(client, title, old).await,
            GivenPluginInstance::Id(id, _) => client
                .get_plugin_instance(id)
                .await
                .map_err(eyre::Error::new),
            GivenPluginInstance::RelativePath(path) => {
                get_relative_path_as_plinst(client, old, path).await
            }
            GivenPluginInstance::AbsolutePath(path) => get_plinst_of_path(client, &path).await,
        }
    }

    pub async fn get_as_path(
        self,
        client: &EitherClient,
        old: Option<PluginInstanceId>,
    ) -> Result<String> {
        match self {
            GivenPluginInstance::Id(id, _) => client
                .get_plugin_instance(id)
                .await
                .map(|p| p.object.output_path)
                .map_err(eyre::Error::new),
            GivenPluginInstance::Title(title) => get_by_title_ro(client, title, old)
                .await
                .map(|p| p.object.output_path),
            GivenPluginInstance::RelativePath(p) => get_relative_path(client, old, &p).await,
            GivenPluginInstance::AbsolutePath(p) => Ok(p),
        }
    }
}

async fn get_relative_path<A: Access, C: BaseChrisClient<A>>(
    client: &C,
    old: Option<PluginInstanceId>,
    rel_path: &str,
) -> Result<String> {
    if let Some(id) = old {
        pwd(client, id, true)
            .await
            .map(|p| reconcile_path(&p, rel_path))
    } else {
        bail!("No current plugin instance context, cannot resolve relative path.")
    }
}

async fn get_relative_path_as_plinst<A: Access, C: BaseChrisClient<A>>(
    client: &C,
    old: Option<PluginInstanceId>,
    rel_path: String,
) -> Result<PluginInstance<A>> {
    if let Some(id) = old {
        let old_output_path = pwd(client, id, true).await?;
        let requested_path = reconcile_path(&old_output_path, &rel_path);
        if let Some(id) = parse_plinst_id(&requested_path) {
            client
                .get_plugin_instance(id)
                .await
                .map_err(eyre::Error::new)
        } else {
            bail!("The relative path {}, canonicalized as {}, is not the output path of a plugin instance.", rel_path, requested_path)
        }
    } else {
        bail!("No current plugin instance context, cannot resolve relative path.")
    }
}

async fn get_plinst_of_path<A: Access, C: BaseChrisClient<A>>(
    client: &C,
    path: &str,
) -> Result<PluginInstance<A>> {
    if let Some(id) = parse_plinst_id(path) {
        client
            .get_plugin_instance(id)
            .await
            .map_err(eyre::Error::new)
    } else {
        bail!("Path could not be understood as a plugin instance.");
    }
}

fn parse_plinst_id(path: &str) -> Option<PluginInstanceId> {
    path.rsplit_once('_')
        .map(|(_, r)| r)
        .and_then(|n| n.parse().ok())
        .map(PluginInstanceId)
}

async fn get_by_title_ro(
    client: &EitherClient,
    name: String,
    old: Option<PluginInstanceId>,
) -> Result<PluginInstanceRo> {
    if let EitherClient::LoggedIn(chris) = client {
        search_title(chris, name, old).await.map(|p| p.into())
    } else {
        bail!("Cannot search for plugin instances without a user account. Please tell Jorge to fix https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/530")
    }
}

async fn search_title(
    chris: &ChrisClient,
    title: String,
    old: Option<PluginInstanceId>,
) -> Result<PluginInstanceRw> {
    if let Some(old) = old {
        if let Some(res) = search_title_within_feed(chris, title.clone(), old).await? {
            return Ok(res);
        }
    }
    search_title_any_feed(chris, title).await
}

async fn search_title_within_feed(
    chris: &ChrisClient,
    title: String,
    old: PluginInstanceId,
) -> Result<Option<PluginInstanceRw>> {
    let old = chris.get_plugin_instance(old).await?;
    let query = chris
        .plugin_instances()
        .feed_id(old.object.feed_id)
        .title(title)
        .page_limit(10)
        .max_items(10);
    let items: Vec<_> = query.search().stream_connected().try_collect().await?;
    if items.len() > 1 {
        bail!(
            "Multiple plugin instances found. Please specify: {}",
            items.iter().map(plugin_instance_string).join(" ")
        );
    }
    Ok(items.into_iter().next())
}

async fn search_title_any_feed(chris: &ChrisClient, title: String) -> Result<PluginInstanceRw> {
    let query = chris.plugin_instances().title(title);
    let items: Vec<_> = query.search().stream_connected().try_collect().await?;
    if items.len() > 1 {
        bail!(
            "Multiple plugin instances found. Please specify: {}",
            items.iter().map(plugin_instance_string).join(" ")
        );
    }
    items
        .into_iter()
        .next()
        .ok_or_eyre("Plugin instance not found")
}

fn plugin_instance_string<A: Access>(p: &LinkedModel<PluginInstanceResponse, A>) -> String {
    format!("plugininstance/{}", p.object.id.0)
}

async fn pwd<A: Access, C: BaseChrisClient<A>>(
    client: &C,
    id: PluginInstanceId,
    strip_data: bool,
) -> Result<String> {
    let output_path = client.get_plugin_instance(id).await?.object.output_path;
    let wd = output_path
        .strip_suffix(if strip_data { "/data" } else { "" })
        .unwrap_or(&output_path)
        .to_string();
    Ok(wd)
}

fn reconcile_path(wd: &str, rel_path: &str) -> String {
    let path = Utf8Path::new(wd).to_path_buf();
    rel_path.split('/').fold(path, reduce_path).to_string()
}

fn reduce_path(acc: Utf8PathBuf, component: &str) -> Utf8PathBuf {
    if component == "." || component.is_empty() {
        acc
    } else if component == ".." {
        acc.parent().map(|p| p.to_path_buf()).unwrap_or(acc)
    } else {
        acc.join(component)
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;

    use super::*;

    #[rstest]
    #[case("hello", "hello")]
    #[case("pi/hello", "hello")]
    #[case("plugininstance/hello", "hello")]
    fn test_given_plugin_instance_is_title(#[case] given: &str, #[case] expected: &str) {
        let actual: GivenPluginInstance = given.to_string().into();
        let expected = GivenPluginInstance::Title(expected.to_string());
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("42", 42)]
    #[case("pi/42", 42)]
    #[case("plugininstance/42", 42)]
    #[case("https://example.org/api/v1/plugins/instances/42/", 42)]
    fn test_given_plugin_instance_is_id(#[case] given: &str, #[case] expected: u32) {
        let actual: GivenPluginInstance = given.to_string().into();
        let expected = GivenPluginInstance::Id(PluginInstanceId(expected), given.to_string());
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case(".")]
    #[case("./")]
    #[case("..")]
    #[case("../")]
    #[case("../other")]
    #[case("../other/")]
    #[case("../..")]
    #[case("../../")]
    #[case("../../")]
    fn test_given_plugin_instance_is_output_path(#[case] given: &str) {
        let actual: GivenPluginInstance = given.to_string().into();
        let expected = GivenPluginInstance::RelativePath(given.to_string());
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("PIPELINES/rudolph/i_am_a_pipeline.yml")]
    #[case("PIPELINES/rudolph/i_am_also_pipeline.yml")]
    #[case("SERVICES/PACS")]
    #[case("SERVICES/PACS/Orthanc/00000_PatientName_000000")]
    #[case("rudolph/feed_130")]
    #[case("rudolph/feed_130/pl-dircopy_543")]
    #[case("rudolph/feed_130/pl-dircopy_543/data")]
    #[case("rudolph/feed_130/pl-dircopy_543/data/output.dat")]
    fn test_given_plugin_instance_is_absolute_path(#[case] given: &str) {
        let actual: GivenPluginInstance = given.to_string().into();
        let expected = GivenPluginInstance::AbsolutePath(given.to_string());
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("a/b/c", ".", "a/b/c")]
    #[case("a/b/c", "./d", "a/b/c/d")]
    #[case("a/b/c", "..", "a/b")]
    #[case("a/b/c", "../", "a/b")]
    #[case("a/b/c", "../..", "a")]
    #[case("a/b/c", "..//..", "a")]
    #[case("a/b/c", "..//..//.", "a")]
    fn test_reconcile_path(#[case] wd: &str, #[case] rel_path: &str, #[case] expected: &str) {
        let actual = reconcile_path(wd, rel_path);
        assert_eq!(&actual, expected)
    }
}
