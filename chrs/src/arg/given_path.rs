use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::{bail, Result};

use chris::types::PluginInstanceId;

use crate::client::RoClient;

/// Resolve a user-supplied optional string argument as a _ChRIS_ file path.
///
/// 1. If user supplied an argument, return it as-is.
/// 2. However, if the user supplied argument contains path components like "../" or "./", join the
///    user-supplied argument with the path of `old_id` and canonicalize the path.
/// 3. If the user did not supply an argument, return the path of `old_id`
pub async fn resolve_optional_path(client: &RoClient, old_id: Option<PluginInstanceId>, path: Option<String>) -> Result<Option<String>> {
    if let Some(p) = path {
        resolve_given_path(client, old_id, p).await.map(Some)
    } else if let Some(id) = old_id {
        pwd(client, id).await.map(Some)
    } else {
        Ok(None)
    }
}


async fn resolve_given_path(
    client: &RoClient,
    pid: Option<PluginInstanceId>,
    given_path: String,
) -> Result<String> {
    if &given_path == "."
        || ["./", "..", "../"]
        .iter()
        .any(|s| given_path.starts_with(s))
    {
        if let Some(id) = pid {
            let wd = pwd(client, id).await?;
            Ok(reconcile_path(&wd, &given_path))
        } else {
            bail!(
                "Cannot cd into {}: no current plugin instance context",
                given_path
            )
        }
    } else {
        Ok(given_path)
    }
}

async fn pwd(client: &RoClient, id: PluginInstanceId) -> Result<String> {
    let output_path = client.get_plugin_instance(id).await?.object.output_path;
    let wd = output_path
        .strip_suffix("/data")
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
