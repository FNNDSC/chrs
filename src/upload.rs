use std::io;
use std::path::PathBuf;
use crate::ChrisClient;


pub fn upload(client: &ChrisClient, files: &Vec<PathBuf>, path: &String) {
    let prefix = PathBuf::from(path);
    let all_files = discover_input_files(files);
    for file in all_files {
        let upload_path = prefix.join(&file).to_string_lossy().into_owned();
        let url = client.upload(&file, &upload_path);
        println!("{}", url);
    }
}


fn discover_input_files(paths: &Vec<PathBuf>) -> Vec<PathBuf> {
    let mut all_files: Vec<PathBuf> = Vec::new();
    for path in paths {
        let mut sub_files = files_under(path);
        all_files.append(&mut sub_files)
    }
    all_files
}


fn files_under(path: &PathBuf) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()]
    }
    match files_under_dir(path) {
        Ok(sub_files) => sub_files,
        Err(e) => panic!("{}", e)
    }
}


fn files_under_dir(dir: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut sub_files: Vec<PathBuf> = Vec::new();
    for entry in dir.read_dir()? {
        let entry = entry?;
        let sub_path = entry.path();
        if sub_path.is_file() {
            sub_files.push(sub_path)
        }
        else {
            let mut nested_files = files_under_dir(&sub_path)?;
            sub_files.append(&mut nested_files);
        }
    }
    Ok(sub_files)
}
