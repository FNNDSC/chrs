use crate::ChrisClient;
use std::io;
use std::path::{Path, PathBuf};

/// Upload local files and directories to my ChRIS Library
pub fn upload(client: &ChrisClient, files: &[PathBuf], path: &str) -> io::Result<()> {
    let prefix = PathBuf::from(path);
    let all_files = discover_input_files(files)?;
    for file in all_files {
        let upload_path = prefix.join(&file).to_string_lossy().into_owned();
        let url = client.upload(&file, &upload_path);
        println!("{}", url);
    }
    Ok(())
}

/// Given a list of files and directories, traverse every directory
/// to obtain just a list of files.
/// Produces Err if any paths are invalid.
fn discover_input_files(paths: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut all_files: Vec<PathBuf> = Vec::new();
    for path in paths {
        let mut sub_files = files_under(path)?;
        all_files.append(&mut sub_files);
    }
    Ok(all_files)
}

/// Get all files under a path, whether the given path is a file or directory.
fn files_under(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {:?}", path),
        ));
    }

    let mut sub_files: Vec<PathBuf> = Vec::new();
    for entry in path.read_dir()? {
        let entry = entry?;
        let sub_path = entry.path();
        if sub_path.is_file() {
            sub_files.push(sub_path)
        } else if sub_path.is_dir() {
            let mut nested_files = files_under(&sub_path)?;
            sub_files.append(&mut nested_files);
        }
    }
    Ok(sub_files)
}

#[cfg(test)]
mod tests {
    use crate::upload::files_under;
    use std::fs::OpenOptions;
    use std::path::Path;
    use std::{fs, io};
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    #[allow(unused_must_use)]
    fn test_files_under_dir() -> io::Result<()> {
        let tmp_dir = TempDir::new()?;
        let tmp_path = tmp_dir.path();
        let nested_dir = tmp_path.join(Path::new("bento/box"));
        let nested_file1_parent = tmp_path.join(Path::new("seaweed/rice"));
        let nested_file1 = nested_file1_parent.join(Path::new("tuna"));
        let nested_file2_parent = tmp_path.join(Path::new("oxygen"));
        let nested_file2 = nested_file2_parent.join(Path::new("o2"));

        fs::create_dir_all(&nested_file1_parent)?;
        fs::create_dir_all(&nested_file2_parent)?;
        fs::create_dir_all(&nested_dir);
        touch(&nested_file1)?;
        touch(&nested_file2)?;

        let actual = files_under(&tmp_path)?;
        assert_eq!(actual.len(), 2);
        assert!(actual.contains(&nested_file1));
        assert!(actual.contains(&nested_file2));
        assert!(!actual.contains(&nested_dir));
        Ok(())
    }

    #[test]
    fn test_files_under_file() -> io::Result<()> {
        let tmp_file = NamedTempFile::new()?;
        let path = tmp_file.path();
        assert_eq!(vec![path.to_path_buf()], files_under(&path)?);
        Ok(())
    }

    #[test]
    fn test_files_under_dne() -> io::Result<()> {
        let path = Path::new("tomato");
        let result = files_under(path);
        assert!(result.is_err());
        let e = result.unwrap_err();
        assert_eq!(io::ErrorKind::NotFound, e.kind());
        assert_eq!(format!("File not found: {:?}", path), e.to_string());
        Ok(())
    }

    /// Create file if it does not exist.
    fn touch(path: &Path) -> io::Result<()> {
        match OpenOptions::new().create(true).write(true).open(path) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
