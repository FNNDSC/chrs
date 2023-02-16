use crate::constants::BUG_REPORTS;
use crate::executor::collect_then_do_with_progress;
use crate::io_helper::progress_bar_bytes;
use anyhow::{bail, Context, Error, Ok, Result};
use chris::common_types::Username;
use chris::errors::CUBEError;
use chris::models::data::{Downloadable, FileUploadResponse, PluginInstanceId};
use chris::{errors::FileIOError, ChrisClient, Pipeline};
use futures::{try_join, TryStreamExt};
use pathdiff::diff_paths;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

/// Upload local files and directories to my ChRIS Library.
///
/// WARNING: uses std::path to iterate over filesystem instead of tokio::fs,
/// meaning that part of its execution is synchronous.
pub async fn upload(
    chris: &ChrisClient,
    files: &[PathBuf],
    upload_path: &str,
    feed: Option<String>,
    pipeline: Option<String>,
) -> Result<()> {
    let feed = feed.or_else(|| pipeline.clone()); // bad clone
    if feed.is_some() && files.len() != 1 && upload_path.is_empty() {
        bail!("A feed can only be created when only one item is specified or when --path is specified.");
    }
    let found_pipeline = get_pipeline(chris, &feed, pipeline).await?;
    let files_to_upload = discover_input_files(files)?;

    let dircopy_dir = if files_to_upload.len() == 1 {
        upload_single(chris, first(files_to_upload).unwrap(), upload_path).await
    } else {
        let uploaded_path = upload_multiple(chris, files_to_upload, upload_path).await?;
        choose_dircopy_path(chris.username(), files, &uploaded_path)
            .context("Upload path unknown --- this is a bug.")
    }?;

    if let Some(feed_name) = feed {
        create_feed(chris, &dircopy_dir, &feed_name, found_pipeline).await?;
    }

    Ok(())
}

fn first<I>(v: Vec<I>) -> Option<I> {
    v.into_iter().next()
}

/// Upload a single file to _ChRIS_, showing a progress bar for the bytes uploaded.
async fn upload_single(
    chris: &ChrisClient,
    file: FileToUpload,
    upload_path: &str,
) -> Result<String> {
    let path = choose_upload_path(chris.username(), file.path.as_path(), upload_path)
        .with_context(|| format!("Unable to decide upload path for {:?}", file.path))?;

    let open_file = File::open(&file.path).await?;
    let stream = FramedRead::new(open_file, BytesCodec::new());
    let content_length = fs::metadata(&file.path).await?.len();
    let bar = progress_bar_bytes(content_length);
    let wrapped_stream = stream.map_ok(move |bytes| {
        bar.inc(bytes.len() as u64);
        bytes
    });

    let upload = chris
        .upload_stream(wrapped_stream, file.name, path, content_length)
        .await?;
    Ok(upload.fname().to_string())
}

/// Upload multiple files to _ChRIS_, showing a progress bar which updates once for every file.
async fn upload_multiple(
    chris: &ChrisClient,
    files_to_upload: Vec<FileToUpload>,
    upload_path: &str,
) -> Result<String> {
    let upload_path = append_slash_if_not_empty(upload_path);
    let uploads = files_to_upload
        .into_iter()
        .map(|file| FileToUpload {
            name: format!("{}{}", upload_path, file.name),
            path: file.path,
        })
        .map(|f| f.upload_using(chris));
    collect_then_do_with_progress(uploads, false).await?;
    Ok(upload_path)
}

async fn get_pipeline(
    chris: &ChrisClient,
    feed: &Option<String>,
    pipeline: Option<String>,
) -> Result<Option<Pipeline>> {
    if feed.is_some() {
        if let Some(pipeline_name) = pipeline {
            let found_pipeline = chris.get_pipeline(&pipeline_name).await?;
            found_pipeline
                .ok_or_else(|| Error::msg(format!("Pipeline not found: \"{}\"", pipeline_name)))
                .map(Some)
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

async fn create_feed(
    chris: &ChrisClient,
    uploaded_dir: &str,
    feed_name: &str,
    pipeline: Option<Pipeline>,
) -> Result<()> {
    let dircopy = chris.dircopy(uploaded_dir).await?;
    let feed = dircopy.feed();
    let feed_name_task = feed.set_name(feed_name);
    let dircopy_id = dircopy.plugin_instance.id;
    let pipeline_task = maybe_create_workflow(pipeline, dircopy_id);
    let (details, _) = try_join!(feed_name_task, pipeline_task)?;
    println!("{}", details.url);
    Ok(())
}

async fn maybe_create_workflow(
    pipeline: Option<Pipeline>,
    previous_plugin_inst_id: PluginInstanceId,
) -> core::result::Result<(), CUBEError> {
    if let Some(p) = pipeline {
        p.create_workflow(previous_plugin_inst_id).await.map(|_| ())
    } else {
        core::result::Result::Ok(())
    }
}

fn choose_dircopy_path(username: &Username, files: &[PathBuf], given_path: &str) -> Option<String> {
    files
        .first()
        .map(|p| p.canonicalize())
        .map(|r| r.ok())
        .unwrap_or(None)
        .as_deref()
        .map(|first_dir| choose_upload_path(username, first_dir, given_path))
        .unwrap_or(None)
}

/// Figure out which path in ChRIS should be used as the argument for pl-dircopy, assuming either:
///
/// - `given_path` is a non-empty string
/// - `files` is a list with a length of exactly one
fn choose_upload_path(username: &Username, first_file: &Path, given_path: &str) -> Option<String> {
    let subdir = if !given_path.is_empty() {
        Some(given_path.to_string())
    } else {
        basename(first_file)
    };
    subdir.map(|d| format!("{}/{}/{}", username, "uploads", d))
}

fn basename(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().to_string())
}

fn append_slash_if_not_empty(s: &str) -> String {
    if s.is_empty() {
        "".to_string()
    } else {
        format!("{}/", s)
    }
}

/// A file on the local filesystem which the user intended to upload into _ChRIS_.
#[derive(PartialEq, Eq, Hash, Debug)]
struct FileToUpload {
    /// Upload target name.
    name: String,
    /// Local path to file.
    path: PathBuf,
}

impl FileToUpload {
    async fn upload_using(self, client: &ChrisClient) -> Result<FileUploadResponse, FileIOError> {
        client.upload_file(&self.path, &self.name).await
    }
}

/// Given a list of files and directories, traverse every directory
/// to obtain just a list of files represented as [FileToUpload].
fn discover_input_files(paths: &[PathBuf]) -> Result<Vec<FileToUpload>> {
    let mut all_files: Vec<FileToUpload> = Vec::new();
    for path in paths {
        let mut sub_files = files_under(path)?;
        all_files.append(&mut sub_files);
    }
    Ok(all_files)
}

/// Get all files under a path as [FileToUpload], where the given
/// path can be either a file or directory.
/// The `name` of results will be their base name, whereas the name
/// of files discovered under a specified directory will be the
/// path relative to the basename of the directory.
fn files_under(path: &Path) -> Result<Vec<FileToUpload>> {
    let canon_path = path
        .canonicalize()
        .with_context(|| format!("File not found: {:?}", path))?;
    if canon_path.is_file() {
        let base = canon_path
            .file_name()
            .with_context(|| format!("Invalid path: {:?}", path))?;
        let file = FileToUpload {
            name: base.to_string_lossy().to_string(),
            path: canon_path,
        };
        return Ok(vec![file]);
    }
    if !canon_path.is_dir() {
        bail!(format!(
            "Path is neither a file nor a directory: {:?}",
            path
        ));
    }
    if canon_path.file_name().is_none() {
        bail!("Unsupported path: {:?}", path);
    }
    let parent = canon_path.parent().unwrap_or(canon_path.as_path());
    files_under_dir(canon_path.as_path(), parent)
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
    fn test_choose_dircopy_path() -> Result<()> {
        let username = Username::from("jack");

        let tmp_dir = TempDir::new()?;
        let given_path = tmp_dir.path().join(Path::new("fruit"));
        fs::create_dir_all(&given_path)?;
        let files = &[given_path];

        assert_eq!(
            choose_dircopy_path(&username, files, ""),
            Some(String::from("jack/uploads/fruit"))
        );
        assert_eq!(
            choose_dircopy_path(&username, files, "vegetables"),
            Some(String::from("jack/uploads/vegetables"))
        );
        Ok(())
    }

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

        std::env::set_current_dir(&pwd)?;
        Ok(())
    }

    /// Create file if it does not exist.
    fn touch(path: &Path) {
        fs::File::create(path).unwrap();
    }
}
