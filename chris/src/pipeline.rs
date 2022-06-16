//! Models for _ChRIS_ file-based representation of pipelines.

pub mod canon;
pub mod rfc2;

pub use canon::{CanonPipeline, ExpandedTreePipeline, PossiblyExpandedTreePipeline};
pub use rfc2::TitleIndexedPipeline;

#[cfg(test)]
mod tests {
    use super::canon::*;
    use super::rfc2::*;
    use lazy_static::lazy_static;
    use rstest::*;
    use std::cmp::Ordering;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;

    lazy_static! {
        static ref EXAMPLE_DIR: &'static Path = Path::new("tests/data/pipelines");
    }

    #[fixture]
    #[once]
    fn canon_linear() -> ExpandedTreePipeline {
        let converted: CanonPipeline =
            read_example_json("fetal_brain_reconstruction_canon.json").into();
        converted.into()
    }

    #[fixture]
    #[once]
    fn json_example_linear() -> ExpandedTreePipeline {
        let converted: CanonPipeline =
            read_example_json("fetal_brain_reconstruction_expanded.json").into();
        converted.into()
    }

    #[fixture]
    #[once]
    fn json_snapshot_branching() -> ExpandedTreePipeline {
        let fname = "fetal_brain_mri_surface_extraction_pipeline.converted.expanded.json";
        let converted: CanonPipeline = read_example_json(fname).into();
        converted.into()
    }

    #[fixture]
    #[once]
    fn yaml_example_linear() -> ExpandedTreePipeline {
        read_example_yaml("fetal_brain_reconstruction.yml")
            .try_into()
            .unwrap()
    }

    #[fixture]
    #[once]
    fn yaml_example_branching() -> ExpandedTreePipeline {
        read_example_yaml("fetal_brain_mri_surface_extraction_pipeline.yml")
            .try_into()
            .unwrap()
    }

    #[rstest]
    fn test_json(canon_linear: &ExpandedTreePipeline, json_example_linear: &ExpandedTreePipeline) {
        assert_eq!(canon_linear, json_example_linear);
    }

    #[rstest]
    fn test_yaml_linear(
        canon_linear: &ExpandedTreePipeline,
        yaml_example_linear: &ExpandedTreePipeline,
    ) {
        cmp_unordered(canon_linear, yaml_example_linear);
    }

    #[rstest]
    fn test_yaml_branching(
        json_snapshot_branching: &ExpandedTreePipeline,
        yaml_example_branching: &ExpandedTreePipeline,
    ) {
        cmp_unordered(json_snapshot_branching, yaml_example_branching);
    }

    fn read_example_json(fname: &str) -> PossiblyExpandedTreePipeline {
        serde_json::from_reader(example_reader(fname)).unwrap()
    }

    fn read_example_yaml(fname: &str) -> TitleIndexedPipeline {
        serde_yaml::from_reader(example_reader(fname)).unwrap()
    }

    fn example_reader(fname: &str) -> BufReader<File> {
        let file = File::open(EXAMPLE_DIR.join(Path::new(fname))).unwrap();
        BufReader::new(file)
    }

    /// a brute-force comparison
    fn cmp_unordered(a: &ExpandedTreePipeline, b: &ExpandedTreePipeline) {
        assert_eq!(a.locked, b.locked);
        assert_eq!(a.name, b.name);
        assert_eq!(a.category, b.category);
        assert_eq!(a.authors, b.authors);
        assert_eq!(a.description, b.description);

        assert_eq!(
            normalize(a.plugin_tree.clone()),
            normalize(b.plugin_tree.clone())
        )
    }

    fn normalize(mut plugin_tree: Vec<ExpandedTreePiping>) -> Vec<ExpandedTreePiping> {
        for piping in plugin_tree.iter_mut() {
            if let Some(params) = &mut piping.plugin_parameter_defaults {
                params.sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
            }
        }
        plugin_tree.sort_by(cmp_pipings);
        plugin_tree
    }

    fn cmp_pipings(a: &ExpandedTreePiping, b: &ExpandedTreePiping) -> Ordering {
        let mut o = a.previous_index.cmp(&b.previous_index);
        if !o.is_eq() {
            return o;
        }
        o = a.plugin_name.as_str().cmp(b.plugin_name.as_str());
        if !o.is_eq() {
            return o;
        }
        o = a.plugin_version.as_str().cmp(b.plugin_version.as_str());
        if !o.is_eq() {
            return o;
        }
        // not comparing plugin_parameter_defaults
        return Ordering::Equal;
    }
}
