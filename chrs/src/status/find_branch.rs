use chris::{PluginInstanceResponse, PluginInstanceRo};
use chris::types::PluginType;

/// For a feed like this:
/// ```
///                 1
///               /  \
///              2    3
///                  / \
///                 4   5
/// ```
///
/// Result: `find_branch_to(4, [1, 2, 3, 4, 5]) -> [1, 3, 4]`
pub(crate) fn find_branch_to<T: PluginInstanceLike>(id: u32, all: &[T]) -> Option<Vec<&T>> {
    all.iter().find(|p| p.id() == id)
        .map(|leaf| loop_up_to_root(leaf, all))
}

fn loop_up_to_root<'a, T: PluginInstanceLike>(leaf: &'a T, all: &'a [T]) -> Vec<&'a T> {
    // I would've preferred recursion, but Rust isn't great for that...
    let mut branch = Vec::with_capacity(all.len());
    let mut head = leaf;
    branch.push(leaf);
    while let Some(previous_id) = head.previous() {
        if head.is_ts() {
            break
        }
        if let Some(previous) = all.iter().find(|p| p.id() == previous_id) {
            head = previous;
            branch.push(previous)
        } else {
            break
        }
    }
    branch.reverse();
    branch
}

pub(crate) trait PluginInstanceLike {
    fn id(&self) -> u32;

    fn previous(&self) -> Option<u32>;

    fn plugin_type(&self) -> PluginType;

    fn is_ts(&self) -> bool {
        matches!(self.plugin_type(), PluginType::Ts)
    }
}

impl PluginInstanceLike for PluginInstanceRo {
    fn id(&self) -> u32 {
        *self.object.id
    }

    fn previous(&self) -> Option<u32> {
        self.object.previous_id.map(|p| p.0)
    }

    fn plugin_type(&self) -> PluginType {
        self.object.plugin_type
    }
}

impl PluginInstanceLike for PluginInstanceResponse {
    fn id(&self) -> u32 {
        *self.id
    }

    fn previous(&self) -> Option<u32> {
        self.previous_id.map(|p| p.0)
    }

    fn plugin_type(&self) -> PluginType {
        self.plugin_type
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rstest::*;

    use chris::PluginInstanceResponse;

    use crate::status::find_branch::find_branch_to;

    #[fixture]
    fn test_data_dir() -> PathBuf {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("test_data");
        d
    }

    #[fixture]
    fn json_string(test_data_dir: PathBuf) -> String {
        let path = test_data_dir.join("cube_chrisproject_org_feed_45_plugininstances_results.json");
        fs_err::read_to_string(path).unwrap()
    }

    #[fixture]
    fn plugin_instances(json_string: String) -> Vec<PluginInstanceResponse> {
        serde_json::from_str(&json_string).unwrap()
    }

    #[rstest]
    fn test_find_branch_to(plugin_instances: Vec<PluginInstanceResponse>) {
        let expected = vec![214, 215, 219, 220];
        let actual_branch = find_branch_to(220, &plugin_instances).unwrap();
        let actual: Vec<_> = actual_branch.into_iter().map(|p| p.id.0).collect();
        assert_eq!(actual, expected)
    }
}
