/// Canonical _ChRIS_ pipeline representations.
use crate::models::{ParameterName, ParameterValue, PluginName, PluginVersion};
use serde::{Deserialize, Serialize};
use std::convert::From;
use aliri_braid::braid;

/// Title of an element of a `plugin_tree` of a
/// [_ChRIS_ RFC #2](https://github.com/FNNDSC/CHRIS_docs/blob/master/rfcs/2-pipeline_yaml.adoc)
/// pipeline.
#[braid(serde)]
pub struct PipingTitle;

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
    #[serde(default = "default_locked")]
    pub locked: bool,
    pub plugin_tree: PossiblyExpandedPluginTree,
}

pub(super) fn default_locked() -> bool {
    true
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PossiblyExpandedPluginTree {
    Expanded(Vec<ExpandedTreePiping>),
    Unexpanded(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ExpandedTreePiping {
    pub title: PipingTitle,
    pub plugin_name: PluginName,
    pub plugin_version: PluginVersion,
    pub previous_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_parameter_defaults: Option<Vec<ExpandedTreeParameter>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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

impl TryFrom<PossiblyExpandedTreePipeline> for ExpandedTreePipeline {
    type Error = serde_json::Error;

    fn try_from(p: PossiblyExpandedTreePipeline) -> Result<Self, Self::Error> {
        let plugin_tree = match p.plugin_tree {
            PossiblyExpandedPluginTree::Expanded(pt) => Ok(pt),
            PossiblyExpandedPluginTree::Unexpanded(plugin_tree) => {
                serde_json::from_str(&plugin_tree)
            }
        }?;
        Ok(ExpandedTreePipeline {
            authors: p.authors,
            name: p.name,
            description: p.description,
            category: p.category,
            locked: p.locked,
            plugin_tree,
        })
    }
}
