//! Simplified, human-friendly pipeline representation as described in
//! [_ChRIS_ RFC #2: _ChRIS_ Pipeline YAML Schema](https://github.com/FNNDSC/CHRIS_docs/blob/master/rfcs/2-pipeline_yaml.adoc).
use super::canon::{default_locked, ExpandedTreeParameter, ExpandedTreePipeline, PipingTitle};
use crate::models::*;
use crate::pipeline::canon::ExpandedTreePiping;
use crate::pipeline::CanonPipeline;
use aliri_braid::braid;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ================================================================================
//                                 DATA DEFINITIONS
// ================================================================================

/// `plugin_name` and `plugin_version` as a single string. See
/// <https://github.com/FNNDSC/CHRIS_docs/blob/master/rfcs/2-pipeline_yaml.adoc#plugin_treeplugin>
#[braid(serde)]
pub struct UnparsedPlugin;

/// A pipeline schema described in
/// [_ChRIS_ RFC #2: _ChRIS_ Pipeline YAML Schema](https://github.com/FNNDSC/CHRIS_docs/blob/master/rfcs/2-pipeline_yaml.adoc).
///
/// A [TitleIndexedPipeline] may or may not be valid. If invalid, it will
/// produce an error when trying to convert it to a [ExpandedTreePipeline].
#[derive(Serialize, Deserialize)]
pub struct TitleIndexedPipeline {
    pub authors: String,
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default = "default_locked")]
    pub locked: bool,
    pub plugin_tree: Vec<TitleIndexedPiping>,
}

/// See [TitleIndexedPipeline].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TitleIndexedPiping {
    pub title: PipingTitle,
    pub plugin: UnparsedPlugin,
    pub previous: Option<PipingTitle>,
    pub plugin_parameter_defaults: Option<HashMap<ParameterName, ParameterValue>>,
}

/// Various explanations for invalid user input.
#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub enum InvalidTitleIndexedPipeline {
    #[error("At lease one element of `plugin_tree` must be the root (i.e. `previous` is null)")]
    NoRoot,
    #[error("Multiple elements of `plugin_tree` found to be the root: {0:?}")]
    PluralRoot(Vec<PipingTitle>),
    #[error("Some `previous` are not connected to `plugin_tree`: {0:?}")]
    Disconnected(HashSet<PipingTitle>),
    #[error(transparent)]
    Plugin(#[from] PluginParseError),
}

// ================================================================================
//                             TOP-LEVEL FUNCTIONALITY
// ================================================================================
//
// To convert from [TitleIndexedPipeline] to [ExpandedTreePipeline] and
// subsequently [CanonPipeline], the bulk of the work is figuring out
// how to convert `plugin_tree`, or more specifically, converting
// [TitleIndexedPiping] into [ExpandedTreePiping]. This involves figuring
// out `previous_index` based on [PipingTitle].

impl TryFrom<TitleIndexedPipeline> for CanonPipeline {
    type Error = InvalidTitleIndexedPipeline;

    fn try_from(value: TitleIndexedPipeline) -> Result<Self, Self::Error> {
        let expanded: ExpandedTreePipeline = value.try_into()?;
        Ok(expanded.into())
    }
}

// ## How It All Works
//
// 1. Convert [TitleIndexedPiping] into either [RootPiping] or [NRPiping],
//    depending on whether `previous == None(..)`
// 2. Create a lookup table mapping the title of each [NRPiping]'s previous
//    to itself.
// 3. Create a list of pipings from the lookup table, while converting
//    pipings to [NumericalPreviousPiping].
// 4. Convert all [NumericalPreviousPiping] to [ExpandedTreePiping] by
//    parsing `plugin_name` and `plugin_version` from plugin.
// 5. Done!

impl TryFrom<TitleIndexedPipeline> for ExpandedTreePipeline {
    type Error = InvalidTitleIndexedPipeline;

    fn try_from(mut p: TitleIndexedPipeline) -> Result<Self, Self::Error> {
        let mut pipings: Vec<NumericPreviousPiping> = Vec::with_capacity(p.plugin_tree.len());
        let root = pop_root(&mut p.plugin_tree)?; // err if more than one root
        let mut non_roots = agg_by_previous(p.plugin_tree);

        pipings.push(root.into());
        pipings.extend(drain_by_previous(&mut non_roots, 0, &pipings[0].title, 1));

        // There will be values remaining inside `non_roots` if there are any
        // elements of `plugin_tree` which specify a `previous` that doesn't exist,
        // i.e. user input is invalid.
        if pipings.len() != pipings.capacity() {
            return Err(InvalidTitleIndexedPipeline::Disconnected(
                non_roots.into_keys().collect(),
            ));
        }

        // parse `plugin_name` and `plugin_version` from `plugin` string
        let plugin_tree: Vec<ExpandedTreePiping> = pipings
            .into_iter()
            .map(ExpandedTreePiping::try_from)
            .try_collect()?;

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

// ================================================================================
//                               HELPER FUNCTIONS
// ================================================================================

// TODO
// #[derive(thiserror::Error, Debug)]
// #[error("`plugin_tree` contains duplicate title: \"{0}\"")]
// struct DuplicateTitle(PipingTitle);

/// Put all the non-root pipings into a map where keys are the `title` of their `previous`.
fn agg_by_previous(plugin_tree: Vec<TitleIndexedPiping>) -> HashMap<PipingTitle, Vec<NRPiping>> {
    let mut m: HashMap<PipingTitle, Vec<NRPiping>> = HashMap::with_capacity(plugin_tree.len());
    let non_roots = plugin_tree
        .into_iter()
        .filter_map(|p| NRPiping::try_from(p).ok());
    for p in non_roots {
        match m.get_mut(&p.previous) {
            None => {
                m.insert(p.previous.clone(), vec![p]);
            }
            Some(v) => {
                v.push(p);
            }
        }
    }
    m
}

/// Remove and return the root of the `plugin_tree`.
fn pop_root(
    plugin_tree: &mut Vec<TitleIndexedPiping>,
) -> Result<RootPiping, InvalidTitleIndexedPipeline> {
    Ok(plugin_tree.swap_remove(index_of_root(plugin_tree)?).into())
}

/// Get the index of the piping which has a null previous.
fn index_of_root(p: &[TitleIndexedPiping]) -> Result<usize, InvalidTitleIndexedPipeline> {
    let roots: Vec<(usize, &PipingTitle)> = p
        .iter()
        .enumerate()
        .filter_map(|(i, piping)| match piping.previous {
            None => Some((i, &piping.title)),
            Some(_) => None,
        })
        .collect();
    match roots.len() {
        1 => Ok(roots[0].0),
        0 => Err(InvalidTitleIndexedPipeline::NoRoot),
        _ => {
            let titles = roots.into_iter().map(|(_, t)| t.clone()).collect();
            Err(InvalidTitleIndexedPipeline::PluralRoot(titles))
        }
    }
}

/// Remove entries from `m` and produce a sequence of them while converting
/// from [NRPiping] to [NumericPreviousPiping] based on the given `previous_index`.
fn drain_by_previous(
    m: &mut HashMap<PipingTitle, Vec<NRPiping>>,
    previous_index: usize,
    previous_title: &PipingTitle,
    current_index: usize,
) -> Vec<NumericPreviousPiping> {
    match m.remove(previous_title) {
        None => vec![],
        Some(children) => {
            let mut child_index = current_index + children.len();
            let mut pipings: Vec<NumericPreviousPiping> = children
                .into_iter()
                .map(|p| p.canonicalize(previous_index))
                .collect(); // siblings
            let mut everything_else: Vec<NumericPreviousPiping> = Vec::new(); // children
            for (i, child) in pipings.iter().enumerate() {
                let grandchildren =
                    drain_by_previous(m, current_index + i, &child.title, child_index);
                child_index += grandchildren.len();
                everything_else.extend(grandchildren);
            }
            pipings.extend(everything_else);
            pipings
        }
    }
}

// ================================================================================
//                             INTERMEDIATE STRUCTS
// ================================================================================

/// A Piping which does not have a previous. There may only be one per pipeline.
struct RootPiping {
    title: PipingTitle,
    plugin: UnparsedPlugin,
    plugin_parameter_defaults: Option<HashMap<ParameterName, ParameterValue>>,
}

/// A Non-Root Piping.
#[derive(Debug, Clone)]
struct NRPiping {
    title: PipingTitle,
    plugin: UnparsedPlugin,
    previous: PipingTitle,
    plugin_parameter_defaults: Option<HashMap<ParameterName, ParameterValue>>,
}

#[derive(Debug, Clone, PartialEq)]
struct NumericPreviousPiping {
    title: PipingTitle,
    plugin: UnparsedPlugin,
    previous_index: Option<usize>,
    plugin_parameter_defaults: Option<HashMap<ParameterName, ParameterValue>>,
}

// ================================================================================
//                            INTERMEDIATE CONVERTERS
// ================================================================================

impl From<TitleIndexedPiping> for RootPiping {
    fn from(p: TitleIndexedPiping) -> Self {
        RootPiping {
            title: p.title,
            plugin: p.plugin,
            plugin_parameter_defaults: p.plugin_parameter_defaults,
        }
    }
}

impl TryFrom<TitleIndexedPiping> for NRPiping {
    type Error = IsRoot;

    fn try_from(p: TitleIndexedPiping) -> Result<Self, Self::Error> {
        match p.previous {
            None => Err(IsRoot),
            Some(previous) => Ok(NRPiping {
                title: p.title,
                plugin: p.plugin,
                previous,
                plugin_parameter_defaults: p.plugin_parameter_defaults,
            }),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("IsRoot")]
struct IsRoot;

impl From<RootPiping> for NumericPreviousPiping {
    fn from(p: RootPiping) -> Self {
        NumericPreviousPiping {
            title: p.title,
            plugin: p.plugin,
            previous_index: None,
            plugin_parameter_defaults: p.plugin_parameter_defaults,
        }
    }
}

impl NRPiping {
    fn canonicalize(self, previous_index: usize) -> NumericPreviousPiping {
        NumericPreviousPiping {
            title: self.title,
            plugin: self.plugin,
            previous_index: Some(previous_index),
            plugin_parameter_defaults: self.plugin_parameter_defaults,
        }
    }
}

impl TryFrom<NumericPreviousPiping> for ExpandedTreePiping {
    type Error = PluginParseError;

    fn try_from(p: NumericPreviousPiping) -> Result<ExpandedTreePiping, Self::Error> {
        let (plugin_name, plugin_version) = parse_plugin(&p.plugin)?;
        Ok(ExpandedTreePiping {
            title: p.title,
            plugin_name,
            plugin_version,
            previous_index: p.previous_index,
            plugin_parameter_defaults: p
                .plugin_parameter_defaults
                .map(|m| m.into_iter().map(|t| t.into()).collect()),
        })
    }
}

impl From<(ParameterName, ParameterValue)> for ExpandedTreeParameter {
    fn from(t: (ParameterName, ParameterValue)) -> Self {
        ExpandedTreeParameter {
            name: t.0,
            default: t.1,
        }
    }
}

fn parse_plugin(plugin: &UnparsedPlugin) -> Result<(PluginName, PluginVersion), PluginParseError> {
    let (utn, version) = plugin
        .as_str()
        .rsplit_once('v')
        .ok_or_else(|| PluginParseError::NoV(plugin.to_owned()))?;
    if !utn.ends_with(' ') {
        return Err(PluginParseError::NoSpaceBeforeV(plugin.to_owned()));
    }
    Ok((
        PluginName::from(utn.trim_end()),
        PluginVersion::from(version),
    ))
}

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub enum PluginParseError {
    #[error("\"{0}\" cannot be parsed as (plugin_name, plugin_version)")]
    NoV(UnparsedPlugin),
    #[error("\"{0}\" cannot be parsed as (plugin_name, plugin_version)")]
    NoSpaceBeforeV(UnparsedPlugin),
}

impl From<NRPiping> for PipingTitle {
    fn from(p: NRPiping) -> Self {
        p.title
    }
}

// ================================================================================
//                                OTHER CONVERTERS
// ================================================================================

impl From<ExpandedTreePipeline> for TitleIndexedPipeline {
    fn from(p: ExpandedTreePipeline) -> Self {
        let plugin_tree = p
            .plugin_tree
            .iter()
            .map(|piping| {
                let plugin = UnparsedPlugin::new(format!(
                    "{} v{}",
                    piping.plugin_name, piping.plugin_version
                ));
                let title = piping.title.clone();
                let previous = piping
                    .previous_index
                    .map(|i| p.plugin_tree[i].title.clone());

                let plugin_parameter_defaults =
                    piping.plugin_parameter_defaults.as_ref().map(|params| {
                        params
                            .iter()
                            .map(|param| (param.name.clone(), param.default.clone()))
                            .collect()
                    });

                TitleIndexedPiping {
                    title,
                    plugin,
                    previous,
                    plugin_parameter_defaults,
                }
            })
            .collect();
        TitleIndexedPipeline {
            authors: p.authors,
            name: p.name,
            description: p.description,
            category: p.category,
            locked: p.locked,
            plugin_tree,
        }
    }
}

// ================================================================================
//                                  UNIT TESTS
// ================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::collections::HashSet;

    #[rstest]
    #[case("pl-dircopy v1.2.3", "pl-dircopy", "1.2.3")]
    #[case("pl-dircopy   v4", "pl-dircopy", "4")]
    #[case("vvvvv v4.5", "vvvvv", "4.5")]
    #[case("vvvv v v4.5", "vvvv v", "4.5")]
    fn test_parse_plugin_ok(#[case] unparsed: &str, #[case] name: &str, #[case] version: &str) {
        assert_eq!(
            parse_plugin(&UnparsedPlugin::from(unparsed)).unwrap(),
            (PluginName::from(name), PluginVersion::from(version))
        );
    }

    #[rstest]
    #[case("pl-dircopyv1.2.3")]
    #[case("pl-dircopy 1.2.3")]
    #[case("pl-dircopyv 1.2.3")]
    fn test_parse_plugin_err(#[case] unparsed: &str) {
        assert!(parse_plugin(&UnparsedPlugin::from(unparsed)).is_err())
    }

    #[rstest]
    fn test_index_of_root() {
        let root1 = create_example("root1", None);
        let root2 = create_example("root2", None);
        let child1 = create_example("child1", Some("root1"));
        let child2 = create_example("root1", Some("child1"));

        assert_eq!(
            index_of_root(&vec![root1.clone(), child1.clone(), child2.clone()]),
            Ok(0)
        );
        assert_eq!(
            index_of_root(&vec![child1.clone(), root1.clone(), child2.clone()]),
            Ok(1)
        );
        assert_eq!(
            index_of_root(&vec![child2.clone(), child1.clone(), root1.clone()]),
            Ok(2)
        );

        let mut plugin_tree = vec![root1.clone(), child1.clone(), child2.clone()];
        let removed_root = pop_root(&mut plugin_tree).unwrap();
        assert_eq!(removed_root.title, root1.title);
        assert_eq!(
            plugin_tree
                .iter()
                .map(|p| &p.title)
                .collect::<HashSet<&PipingTitle>>(),
            vec![&child1.title, &child2.title]
                .into_iter()
                .collect::<HashSet<&PipingTitle>>()
        );

        assert_eq!(
            index_of_root(&vec![child1.clone(), child2.clone()]).unwrap_err(),
            InvalidTitleIndexedPipeline::NoRoot
        );
        assert_eq!(
            index_of_root(&vec![
                root1.clone(),
                child1.clone(),
                root2.clone(),
                child2.clone()
            ])
            .unwrap_err(),
            InvalidTitleIndexedPipeline::PluralRoot(vec![root1.title, root2.title])
        );
    }

    #[rstest]
    fn test_index_by_previous() {
        let examples = vec![
            create_example("example1", None),
            create_example("example2", None),
            create_example("example3", Some("a")),
            create_example("example4", Some("b")),
            create_example("example5", Some("b")),
        ];
        let mut m = agg_by_previous(examples);
        assert_set_title_eq(m.remove(&PipingTitle::from("a")).unwrap(), vec!["example3"]);
        assert_set_title_eq(
            m.remove(&PipingTitle::from("b")).unwrap(),
            vec!["example4", "example5"],
        );
        assert!(m.is_empty());
    }

    #[rstest]
    fn test_drain_by_previous_empty() {
        assert!(drain_by_previous(&mut HashMap::new(), 0, &PipingTitle::from("lol"), 1).is_empty());
    }

    #[rstest]
    fn test_drain_by_previous_linear() {
        let mut m: HashMap<PipingTitle, Vec<NRPiping>> = HashMap::new();
        let a: NRPiping = create_example("a", Some("root")).try_into().unwrap();
        let b: NRPiping = create_example("b", Some("a")).try_into().unwrap();
        let c: NRPiping = create_example("c", Some("b")).try_into().unwrap();
        m.insert(PipingTitle::from("root"), vec![a.clone()]);
        m.insert(PipingTitle::from("a"), vec![b.clone()]);
        m.insert(PipingTitle::from("b"), vec![c.clone()]);

        let expected = vec![a.canonicalize(0), b.canonicalize(1), c.canonicalize(2)];
        let actual = drain_by_previous(&mut m, 0, &PipingTitle::from("root"), 1);
        assert_eq!(actual, expected);
    }

    #[rstest]
    fn test_drain_by_previous_branching() {
        //       root
        //       / \
        //      /   \
        //     a     b
        //   / | \   | \
        //  c  d  e  f  g
        //    / \      / \
        //   h   i    j   k
        let mut m: HashMap<PipingTitle, Vec<NRPiping>> = HashMap::new();
        let a: NRPiping = create_example("a", Some("root")).try_into().unwrap();
        let b: NRPiping = create_example("b", Some("root")).try_into().unwrap();
        let c: NRPiping = create_example("c", Some("a")).try_into().unwrap();
        let d: NRPiping = create_example("d", Some("a")).try_into().unwrap();
        let e: NRPiping = create_example("e", Some("a")).try_into().unwrap();
        let f: NRPiping = create_example("f", Some("b")).try_into().unwrap();
        let g: NRPiping = create_example("g", Some("b")).try_into().unwrap();
        let h: NRPiping = create_example("h", Some("d")).try_into().unwrap();
        let i: NRPiping = create_example("i", Some("d")).try_into().unwrap();
        let j: NRPiping = create_example("j", Some("g")).try_into().unwrap();
        let k: NRPiping = create_example("k", Some("g")).try_into().unwrap();
        m.insert(PipingTitle::from("root"), vec![a.clone(), b.clone()]);
        m.insert(
            PipingTitle::from("a"),
            vec![c.clone(), d.clone(), e.clone()],
        );
        m.insert(PipingTitle::from("b"), vec![f.clone(), g.clone()]);
        m.insert(PipingTitle::from("d"), vec![h.clone(), i.clone()]);
        m.insert(PipingTitle::from("g"), vec![j.clone(), k.clone()]);

        let expected = vec![
            a.canonicalize(0), //  1
            b.canonicalize(0), //  2
            c.canonicalize(1), //  3
            d.canonicalize(1), //  4
            e.canonicalize(1), //  5
            h.canonicalize(4), //  6
            i.canonicalize(4), //  7
            f.canonicalize(2), //  8
            g.canonicalize(2), //  9
            j.canonicalize(9), // 10
            k.canonicalize(9), // 11
        ];
        let actual = drain_by_previous(&mut m, 0, &PipingTitle::from("root"), 1);
        assert_eq!(actual, expected);
    }

    #[rstest]
    fn test_no_root() {
        let pipeline = create_pipeline(vec![
            create_example("a", Some("something")),
            create_example("b", Some("something")),
        ]);
        assert_eq!(pipeline.unwrap_err(), InvalidTitleIndexedPipeline::NoRoot);
    }

    #[rstest]
    fn test_previous_not_found() {
        let pipeline = create_pipeline(vec![
            create_example("root", None),
            create_example("child", Some("root")),
            create_example("orphan", Some("dne")),
        ]);
        assert_eq!(
            pipeline.unwrap_err(),
            InvalidTitleIndexedPipeline::Disconnected([PipingTitle::from("dne")].into())
        );
    }

    #[rstest]
    fn test_has_plural_roots() {
        let pipeline = create_pipeline(vec![
            create_example("root", None),
            create_example("child", Some("root")),
            create_example("another_root", None),
        ]);
        assert_eq!(
            pipeline.unwrap_err(),
            InvalidTitleIndexedPipeline::PluralRoot(vec![
                PipingTitle::from("root"),
                PipingTitle::from("another_root")
            ])
        );
    }

    #[rstest]
    fn test_cycle() {
        let pipeline = create_pipeline(vec![
            create_example("a", None),
            create_example("b", Some("c")),
            create_example("c", Some("b")),
        ]);
        assert_eq!(
            pipeline.unwrap_err(),
            InvalidTitleIndexedPipeline::Disconnected(
                [PipingTitle::from("c"), PipingTitle::from("b")].into()
            )
        );
    }

    fn create_example(title: &str, previous: Option<&str>) -> TitleIndexedPiping {
        TitleIndexedPiping {
            title: PipingTitle::from(title),
            plugin: UnparsedPlugin::from(format!("pl-{} v0.0.0", title)),
            previous: previous.map(PipingTitle::from),
            plugin_parameter_defaults: None,
        }
    }

    fn create_pipeline(
        plugin_tree: Vec<TitleIndexedPiping>,
    ) -> Result<ExpandedTreePipeline, InvalidTitleIndexedPipeline> {
        TitleIndexedPipeline {
            authors: "Me <dev@babyMRI.org>".to_string(),
            name: "Very Fun".to_string(),
            description: "Broken pipeline example for testing chris-rs".to_string(),
            category: "Test Example".to_string(),
            locked: true,
            plugin_tree,
        }
        .try_into()
    }

    fn assert_set_title_eq(left: Vec<impl Into<PipingTitle>>, right: Vec<&str>) {
        let left_set: HashSet<PipingTitle> = left.into_iter().map_into().collect();
        let right_set: HashSet<PipingTitle> = right.into_iter().map(PipingTitle::from).collect();
        assert_eq!(left_set, right_set);
    }
}
