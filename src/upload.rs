use std::io;
use std::path::PathBuf;
use crate::ChrisClient;


/// Upload local files and directories to my ChRIS Library
pub fn upload(client: &ChrisClient, files: &Vec<PathBuf>, path: &String) {
    let prefix = PathBuf::from(path);
    let all_files = discover_input_files(files);
    for file in all_files {
        let upload_path = prefix.join(&file).to_string_lossy().into_owned();
        let url = client.upload(&file, &upload_path);
        println!("{}", url);
    }
}


/// Given a list of files and directories, traverse every directory
/// to obtain just a list of files.
fn discover_input_files(paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut all_files: Vec<PathBuf> = Vec::new();
    for path in paths {
        match files_under(path) {
            Ok(mut sub_files) => all_files.append(&mut sub_files),
            Err(e) => panic!("{}", e)
        }
    }
    all_files
}


/// Get all files under a path, whether the given path is a file or directory.
fn files_under(path: &PathBuf) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()])
    }

    let mut sub_files: Vec<PathBuf> = Vec::new();
    for entry in path.read_dir()? {
        let entry = entry?;
        let sub_path = entry.path();
        if sub_path.is_file() {
            sub_files.push(sub_path)
        }
        else {
            let mut nested_files = files_under(&sub_path)?;
            sub_files.append(&mut nested_files);
        }
    }
    Ok(sub_files)
}
