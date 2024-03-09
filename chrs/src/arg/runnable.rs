use color_eyre::eyre;
use color_eyre::eyre::bail;
use color_eyre::owo_colors::OwoColorize;
use futures::TryStreamExt;
use std::str::FromStr;

use crate::shlex::shlex_quote;
use chris::search::PluginSearchBuilder;
use chris::types::{CubeUrl, PipelineId, PluginId};
use chris::{Access, BaseChrisClient, LinkedModel, PipelineResponse, PluginResponse, RoAccess};

/// A `GivenRunnable` is a user-provided value representing a plugin or pipeline.
#[derive(Debug, PartialEq, Clone)]
pub enum GivenRunnable {
    PluginId {
        id: PluginId,
        original: String,
    },
    PluginName {
        name: String,
        version: Option<String>,
        original: String,
    },
    PipelineId {
        id: PipelineId,
        original: String,
    },
    PipelineName(String),
}

#[derive(thiserror::Error, Debug)]
#[error("Empty string")]
pub struct GivenRunnableEmptyError;

impl TryFrom<String> for GivenRunnable {
    type Error = GivenRunnableEmptyError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        if value.is_empty() {
            Err(GivenRunnableEmptyError)
            // 1) try to parse as a URL
        } else if let Some(given_plugin) = parse_plugin_id_from_url(&value) {
            Ok(given_plugin)
        } else if let Some(given_pipeline) = parse_pipeline_id_from_url(&value) {
            Ok(given_pipeline)
        } else if let Some(right) = value
            // 2) try to parse as a qualified plugin
            .strip_prefix("pl/")
            .or_else(|| value.strip_prefix("plugin/"))
        {
            Ok(parse_plugin_name_or_id(right.to_string()))
        } else if let Some(right) = value
            // 3) try to parse as a qualified pipeline
            .strip_prefix("pp/")
            .or_else(|| value.strip_prefix("pipeline/"))
        {
            Ok(parse_pipeline_name_or_id(right.to_string()))
            // 4) assume space-containing string is a pipeline name
        } else if value.contains(' ') {
            Ok(Self::PipelineName(value))
        } else {
            // 5) assume is a plugin name. Plugin names do not usually contain spaces.
            Ok(parse_plugin_name_or_id(value))
        }
    }
}

fn parse_plugin_name_or_id(original: String) -> GivenRunnable {
    if let Ok(id) = original.parse().map(PluginId) {
        GivenRunnable::PluginId { id, original }
    } else {
        parse_plugin_and_version(original)
    }
}

fn parse_plugin_id_from_url(original: &str) -> Option<GivenRunnable> {
    original
        .rsplit_once("plugins/")
        .and_then(|(left, right)| CubeUrl::from_str(left).ok().map(|_| right))
        .and_then(|right| right.strip_suffix('/'))
        .and_then(|part| part.parse().ok())
        .map(|num| PluginId(num))
        .map(|id| GivenRunnable::PluginId {
            id,
            original: original.to_string(),
        })
}

fn parse_pipeline_id_from_url(original: &str) -> Option<GivenRunnable> {
    original
        .rsplit_once("pipelines/")
        .and_then(|(left, right)| CubeUrl::from_str(left).ok().map(|_| right))
        .and_then(|right| right.strip_suffix('/'))
        .and_then(|part| part.parse().ok())
        .map(|num| PipelineId(num))
        .map(|id| GivenRunnable::PipelineId {
            id,
            original: original.to_string(),
        })
}

fn parse_pipeline_name_or_id(original: String) -> GivenRunnable {
    if let Ok(id) = original.parse().map(PipelineId) {
        GivenRunnable::PipelineId { id, original }
    } else {
        GivenRunnable::PipelineName(original)
    }
}

fn parse_plugin_and_version(value: String) -> GivenRunnable {
    if let Some((name, version)) = value.rsplit_once('@') {
        GivenRunnable::PluginName {
            name: name.to_string(),
            version: Some(version.to_string()),
            original: value,
        }
    } else {
        GivenRunnable::PluginName {
            name: value.to_string(),
            version: None,
            original: value,
        }
    }
}

impl GivenRunnable {
    pub fn as_arg_str(&self) -> &str {
        match self {
            GivenRunnable::PluginId { original, .. } => original,
            GivenRunnable::PluginName { original, .. } => original,
            GivenRunnable::PipelineId { original, .. } => original,
            GivenRunnable::PipelineName(name) => name,
        }
    }

    pub async fn resolve_using<A: Access, C: BaseChrisClient<A> + Sync>(
        self,
        client: &C,
    ) -> eyre::Result<Runnable<A>> {
        match self {
            GivenRunnable::PluginId { id, .. } => client
                .get_plugin(id)
                .await
                .map(Runnable::Plugin)
                .map_err(eyre::Error::new),
            GivenRunnable::PluginName { name, version, .. } => {
                get_one_plugin_by_name(client, name, version)
                    .await
                    .map(Runnable::Plugin)
            }
            GivenRunnable::PipelineId { id, .. } => client
                .get_pipeline(id)
                .await
                .map(Runnable::Pipeline)
                .map_err(eyre::Error::new),
            GivenRunnable::PipelineName(name) => get_one_pipeline_by_name(client, name)
                .await
                .map(Runnable::Pipeline),
        }
    }
}

async fn get_one_plugin_by_name<A: Access, C: BaseChrisClient<A> + Sync>(
    client: &C,
    name: String,
    version: Option<String>,
) -> eyre::Result<LinkedModel<PluginResponse, A>> {
    let query = plugin_search_query(client, &name, version.as_deref());
    let search = query.search();
    if let Some(plugin) = search.get_first().await? {
        Ok(plugin)
    } else {
        bail!(
            "Plugin not found: {}",
            plugin_to_string(&name, version.as_deref())
        )
    }
}

/// Create a plugin search query returning one result with `name_exact` and maybe `version`.
fn plugin_search_query<'a, A: Access, C: BaseChrisClient<A> + Sync>(
    client: &'a C,
    name: &'a str,
    version: Option<&'a str>,
) -> PluginSearchBuilder<'a, A> {
    let name_query = client.plugin().name_exact(name).page_limit(1).max_items(1);
    if let Some(version) = version {
        name_query.version(version)
    } else {
        name_query
    }
}

fn plugin_to_string(name: &str, version: Option<&str>) -> String {
    if let Some(version) = version {
        format!("{}@{}", name, version)
    } else {
        name.to_string()
    }
}

async fn get_one_pipeline_by_name<A: Access, C: BaseChrisClient<A> + Sync>(
    client: &C,
    name: String,
) -> eyre::Result<LinkedModel<PipelineResponse, A>> {
    // Pipeline search API does not have a `name_exact` field.
    // https://github.com/FNNDSC/ChRIS_ultron_backEnd/issues/539
    let query = client.pipeline().name(&name).page_limit(2).max_items(2);
    let search = query.search();
    let results: Vec<_> = search.stream_connected().try_collect().await?;
    if results.len() > 1 {
        let cmd = format!("chrs search {}", shlex_quote(&name));
        bail!("Multiple pipelines found, please be more specific. Try searching for pipelines by running `{}`, and then rerun this command but specify a pipeline/{}", cmd.bold(), "ID".bold().bright_green())
    };
    if let Some(pipeline) = results.into_iter().next() {
        Ok(pipeline)
    } else {
        bail!("Pipeline not found")
    }
}

/// A `Runnable` is a [GivenRunnable] which was resolved to an existing plugin or pipeline in CUBE.
pub enum Runnable<A: Access> {
    Plugin(LinkedModel<PluginResponse, A>),
    Pipeline(LinkedModel<PipelineResponse, A>),
}

impl<A: Access> Runnable<A>
where
    LinkedModel<PluginResponse, RoAccess>: From<LinkedModel<PluginResponse, A>>,
    LinkedModel<PipelineResponse, RoAccess>: From<LinkedModel<PipelineResponse, A>>,
{
    pub fn into_ro(self) -> Runnable<RoAccess> {
        match self {
            Runnable::Plugin(p) => Runnable::Plugin(p.into()),
            Runnable::Pipeline(p) => Runnable::Pipeline(p.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_given_runnable_cannot_be_empty() {
        assert!(GivenRunnable::try_from("".to_string()).is_err())
    }

    #[rstest]
    #[case("pl-dcm2niix", "pl-dcm2niix", None)]
    #[case("pl-dcm2niix@1.0.0", "pl-dcm2niix", Some("1.0.0"))]
    #[case("pl/pl-dcm2niix", "pl-dcm2niix", None)]
    #[case("pl/pl-dcm2niix@1.0.0", "pl-dcm2niix", Some("1.0.0"))]
    #[case("plugin/pl-dcm2niix@1.0.0", "pl-dcm2niix", Some("1.0.0"))]
    fn test_parse_plugin_name(
        #[case] input: &str,
        #[case] name: &str,
        #[case] version: Option<&str>,
    ) {
        let expected = GivenRunnable::PluginName {
            name: name.to_string(),
            version: version.map(|v| v.to_string()),
            original: plugin_to_string(name, version),
        };
        let actual = GivenRunnable::try_from(input.to_string()).unwrap();
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("42", 42)]
    #[case("pl/42", 42)]
    #[case("plugin/42", 42)]
    fn test_parse_plugin_id(#[case] input: &str, #[case] expected: u32) {
        let expected = GivenRunnable::PluginId {
            id: PluginId(expected),
            original: expected.to_string(),
        };
        let actual = GivenRunnable::try_from(input.to_string()).unwrap();
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("Brain processing", "Brain processing")]
    #[case("pp/Brain processing", "Brain processing")]
    #[case("pipeline/Brain processing", "Brain processing")]
    fn test_parse_pipeline_name(#[case] input: &str, #[case] expected: &str) {
        let expected = GivenRunnable::PipelineName(expected.to_string());
        let actual = GivenRunnable::try_from(input.to_string()).unwrap();
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("pp/42", 42)]
    #[case("pipeline/42", 42)]
    fn test_parse_pipeline_id(#[case] input: &str, #[case] expected: u32) {
        let expected = GivenRunnable::PipelineId {
            id: PipelineId(expected),
            original: expected.to_string(),
        };
        let actual = GivenRunnable::try_from(input.to_string()).unwrap();
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("https://example.com/api/v1/plugins/42/", 42)]
    #[case("https://example.com/api/v1/plugins/560/", 560)]
    fn test_parse_plugin_url(#[case] input: &str, #[case] expected: u32) {
        let expected = GivenRunnable::PluginId {
            id: PluginId(expected),
            original: input.to_string(),
        };
        let actual = GivenRunnable::try_from(input.to_string()).unwrap();
        assert_eq!(actual, expected)
    }

    #[rstest]
    #[case("https://example.com/api/v1/pipelines/42/", 42)]
    #[case("https://example.com/api/v1/pipelines/560/", 560)]
    fn test_parse_pipeline_url(#[case] input: &str, #[case] expected: u32) {
        let expected = GivenRunnable::PipelineId {
            id: PipelineId(expected),
            original: input.to_string(),
        };
        let actual = GivenRunnable::try_from(input.to_string()).unwrap();
        assert_eq!(actual, expected)
    }
}
