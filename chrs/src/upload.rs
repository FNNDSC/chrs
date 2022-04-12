use crate::constants::BUG_REPORTS;
use anyhow::{bail, Context, Error, Ok, Result};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};

/// Upload local files and directories to my ChRIS Library.
///
/// WARNING: uses std::path to iterate over filesystem instead of tokio::fs
pub(crate) async fn upload(files: &[PathBuf], path: &str) -> Result<()> {
    println!("files={:?}, path={:?}", files, path);
    let all_files = discover_input_files(files)?;
    for file in all_files {
        let upload_path = format!("{}/{}", path, file.name);
        println!("{:?} -> {:?}", file.path, upload_path);
    }
    bail!("Not implemented anymore")
}

/// A file on the local filesystem which the user intended to upload into _ChRIS_.
#[derive(PartialEq, Eq, Hash, Debug)]
struct FileToUpload {
    /// Upload target name.
    name: String,
    /// Local path to file.
    path: PathBuf,
}

/// Given a list of files and directories, traverse every directory
/// to obtain just a list of files represented as [FileToUpload].
fn discover_input_files(paths: &[PathBuf]) -> Result<Vec<FileToUpload>> {
    let mut all_files: Vec<FileToUpload> = Vec::new();
    for path in paths {
        let mut sub_files = files_under(path)?;
        all_files.append(&mut sub_files);
    }
    // Ok(all_files)
    todo!()
}

/// Get all files under a path as [FileToUpload], where the given
/// path can be either a file or directory.
/// The `name` of results will be their base name, whereas the name
/// of files discovered under a specified directory will be the
/// path relative to the basename of the directory.
fn files_under(path: &Path) -> Result<Vec<FileToUpload>> {
    if path.is_file() {
        let base = path
            .file_name()
            .with_context(|| format!("Invalid path: {:?}", path))?;
        let file = FileToUpload {
            name: base.to_string_lossy().to_string(),
            path: path.to_path_buf(),
        };
        return Ok(vec![file]);
    }
    if !path.is_dir() {
        bail!(format!("File not found: {:?}", path));
    }
    if path.file_name().is_none() {
        // it's too hard to figure out the appropriate name for parent paths such as ".." or "../.."
        bail!("Unsupported path: {:?}", path);
    }
    let parent = path.parent().unwrap_or(path);
    files_under_dir(path, parent)
}

fn files_under_dir(dir: &Path, parent: &Path) -> Result<Vec<FileToUpload>> {
    let mut sub_files: Vec<FileToUpload> = Vec::new();
    for entry in dir.read_dir()? {
        let entry = entry?;
        let sub_path = entry.path();
        if sub_path.is_file() {
            let name = diff_paths(&sub_path, parent)
                .ok_or_else(|| {
                    Error::msg(format!(
                        "{:?} not found under {:?}\
                \nPlease report this bug: {}",
                        &sub_path, parent, BUG_REPORTS
                    ))
                })?
                .to_string_lossy()
                .to_string();
            let file = FileToUpload {
                name,
                path: sub_path,
            };
            sub_files.push(file)
        } else if sub_path.is_dir() {
            let mut nested_files = files_under_dir(&sub_path, parent)?;
            sub_files.append(&mut nested_files);
        }
    }
    Ok(sub_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_files_under_file() -> Result<()> {
        let tmp_file = NamedTempFile::new()?;
        let path = tmp_file.path();
        let expected = FileToUpload {
            name: path.file_name().unwrap().to_string_lossy().to_string(),
            path: path.to_path_buf(),
        };
        assert_eq!(vec![expected], files_under(&path)?);
        Ok(())
    }

    #[test]
    #[allow(unused_must_use)]
    fn test_files_under_dir() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let given_path = tmp_dir.path().join(Path::new("japan"));
        let nested_file1_parent = given_path.join(Path::new("seaweed/rice"));
        let nested_file1 = nested_file1_parent.join(Path::new("tuna"));
        let nested_file2_parent = given_path.join(Path::new("oxygen"));
        let nested_file2 = nested_file2_parent.join(Path::new("o2"));
        let nested_dir = given_path.join(Path::new("bento/box"));

        fs::create_dir_all(&nested_file1_parent)?;
        fs::create_dir_all(&nested_file2_parent)?;
        fs::create_dir_all(&nested_dir);
        touch(&nested_file1);
        touch(&nested_file2);

        let actual = files_under(&given_path)?;
        let expected = HashSet::from([
            FileToUpload {
                name: "japan/seaweed/rice/tuna".to_string(),
                path: nested_file1.clone(),
            },
            FileToUpload {
                name: "japan/oxygen/o2".to_string(),
                path: nested_file2.clone(),
            },
        ]);
        assert_eq!(
            actual.into_iter().collect::<HashSet<FileToUpload>>(),
            expected
        );
        Ok(())
    }

    #[test]
    fn test_files_under_dne() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let path = tmp_dir.path().join(Path::new("tomato"));
        let result = files_under(&path);
        assert!(result.is_err());
        let e = result.unwrap_err();
        assert_eq!(format!("File not found: {:?}", path), e.to_string());
        Ok(())
    }

    #[test]
    fn test_from_parent() -> Result<()> {
        let tmp_dir = TempDir::new()?;
        let given_path = tmp_dir.path().join("japan");
        let nested_dir = given_path.join("sesame/seaweed/rice/tuna");
        let file = given_path.join("sesame/seaweed/filling");

        fs::create_dir_all(&nested_dir)?;
        touch(&file);

        let pwd = std::env::current_dir()?;
        std::env::set_current_dir(&nested_dir)?;
        assert_eq!(
            files_under(Path::new("../../filling"))?[0].name.as_str(),
            "filling"
        );
        assert_eq!(
            files_under(Path::new("../../../seaweed"))?[0].name.as_str(),
            "seaweed/filling"
        );
        assert_eq!(
            files_under(Path::new("../.."))
                .unwrap_err()
                .to_string()
                .as_str(),
            "Unsupported path: \"../..\""
        );

        std::env::set_current_dir(&pwd)?;
        Ok(())
    }

    /// Create file if it does not exist.
    fn touch(path: &Path) {
        fs::File::create(path).unwrap();
    }
}
