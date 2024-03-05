use chris::types::PluginInstanceId;

/// A user-provided string which is supposed to refer to an existing plugin instance.
#[derive(Debug, PartialEq, Clone)]
pub enum GivenPluginInstance {
    Title(String),
    Id(PluginInstanceId),
    Parent(u32),
}

impl From<String> for GivenPluginInstance {
    fn from(value: String) -> Self {
        if let Some(count) = parse_parent_dirs(&value) {
            return GivenPluginInstance::Parent(count);
        }
        if let Some(id) = parse_id_from_url(&value) {
            return GivenPluginInstance::Id(id);
        }
        let right_part = if let Some((left, right)) = value.split_once('/') {
            if left == "pi" || left == "plugininstance" {
                right
            } else {
                &value
            }
        } else {
            value.as_str()
        };
        right_part
            .parse::<u32>()
            .map(PluginInstanceId)
            .map(GivenPluginInstance::Id)
            .unwrap_or_else(|_e| GivenPluginInstance::Title(right_part.to_string()))
    }
}

fn parse_parent_dirs(value: &str) -> Option<u32> {
    let mut count = 0;
    for part in value.split('/') {
        if part.is_empty() {
            return not_zero(count);
        }
        if part != ".." {
            return None;
        }
        count += 1;
    }
    not_zero(count)
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

fn not_zero(value: u32) -> Option<u32> {
    if value == 0 {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rstest::*;

    #[rstest]
    #[case("hello", GivenPluginInstance::Title("hello".to_string()))]
    #[case("pi/hello", GivenPluginInstance::Title("hello".to_string()))]
    #[case("plugininstance/hello", GivenPluginInstance::Title("hello".to_string()))]
    #[case("42", GivenPluginInstance::Id(PluginInstanceId(42)))]
    #[case("pi/42", GivenPluginInstance::Id(PluginInstanceId(42)))]
    #[case("plugininstance/42", GivenPluginInstance::Id(PluginInstanceId(42)))]
    #[case(
        "https://example.org/api/v1/plugins/instances/42/",
        GivenPluginInstance::Id(PluginInstanceId(42))
    )]
    #[case("..", GivenPluginInstance::Parent(1))]
    #[case("../", GivenPluginInstance::Parent(1))]
    #[case("../..", GivenPluginInstance::Parent(2))]
    #[case("../../", GivenPluginInstance::Parent(2))]
    fn test_given_plugin_instance(#[case] given: &str, #[case] expected: GivenPluginInstance) {
        let actual: GivenPluginInstance = given.to_string().into();
        assert_eq!(actual, expected)
    }
}
