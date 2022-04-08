use crate::api::{ParameterName, ParameterValue, PluginName, PluginVersion};
use serde::{Deserialize, Serialize};
use std::convert::From;

/// A pipeline the way CUBE wants it (where `plugin_tree` is a string).
#[derive(Serialize, Debug, PartialEq)]
pub struct CanonPipeline {
    pub authors: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub locked: bool,
    pub plugin_tree: String,
}

/// A pipeline representation which is the same as [CanonPipeline],
/// but where `plugin_tree` **might be** a deserialized object instead of a string.
/// User input files can be loaded as a [PossiblyExpandedTreePipeline] and converted
/// into [CanonPipeline] or [ExpandedTreePipeline].
#[derive(Debug, Serialize, Deserialize)]
pub struct PossiblyExpandedTreePipeline {
    pub authors: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub locked: bool,
    pub plugin_tree: PossiblyExpandedPluginTree,
}

/// A pipeline representation which is the same as [CanonPipeline],
/// but where `plugin_tree` **is** an object.
///
/// [ExpandedTreePipeline] is easier to work with than [CanonPipeline],
/// but unlike [CanonPipeline], a [ExpandedTreePipeline] cannot be
/// uploaded to CUBE.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ExpandedTreePipeline {
    pub authors: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub locked: bool,
    pub plugin_tree: Vec<ExpandedTreePiping>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PossiblyExpandedPluginTree {
    Expanded(Vec<ExpandedTreePiping>),
    Unexpanded(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ExpandedTreePiping {
    pub plugin_name: PluginName,
    pub plugin_version: PluginVersion,
    pub previous_index: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_parameter_defaults: Option<Vec<ExpandedTreeParameter>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ExpandedTreeParameter {
    pub name: ParameterName,
    pub default: ParameterValue,
}

impl From<ExpandedTreePipeline> for CanonPipeline {
    fn from(p: ExpandedTreePipeline) -> Self {
        CanonPipeline {
            authors: p.authors,
            name: p.name,
            description: p.description,
            category: p.category,
            locked: p.locked,
            plugin_tree: serde_json::to_string(&p.plugin_tree).unwrap(),
        }
    }
}

impl From<PossiblyExpandedTreePipeline> for CanonPipeline {
    fn from(p: PossiblyExpandedTreePipeline) -> Self {
        let plugin_tree = match p.plugin_tree {
            PossiblyExpandedPluginTree::Unexpanded(t) => t,
            PossiblyExpandedPluginTree::Expanded(t) => serde_json::to_string(&t).unwrap(),
        };
        CanonPipeline {
            authors: p.authors,
            name: p.name,
            description: p.description,
            category: p.category,
            locked: p.locked,
            plugin_tree,
        }
    }
}

impl From<CanonPipeline> for ExpandedTreePipeline {
    fn from(p: CanonPipeline) -> Self {
        let plugin_tree: Vec<ExpandedTreePiping> = serde_json::from_str(&p.plugin_tree).unwrap();
        ExpandedTreePipeline {
            authors: p.authors,
            name: p.name,
            description: p.description,
            category: p.category,
            locked: p.locked,
            plugin_tree,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    lazy_static! {
        static ref EXAMPLE_DIR: &'static Path = Path::new("tests/data/pipelines");
    }

    #[test]
    fn test_serialization() {
        let canon_converted: CanonPipeline =
            read_example_json("fetal_brain_reconstruction_canon.json")
                .unwrap()
                .into();
        let expanded_converted: CanonPipeline =
            read_example_json("fetal_brain_reconstruction_expanded.json")
                .unwrap()
                .into();

        let canon_expanded: ExpandedTreePipeline = canon_converted.into();
        let expanded_expanded: ExpandedTreePipeline = expanded_converted.into();

        assert_eq!(canon_expanded, expanded_expanded);
    }

    fn read_example_json(
        fname: &str,
    ) -> Result<PossiblyExpandedTreePipeline, Box<dyn std::error::Error>> {
        let file = File::open(EXAMPLE_DIR.join(Path::new(fname)))?;
        let reader = BufReader::new(file);
        let x = serde_json::from_reader(reader)?;
        Ok(x)
    }
}
