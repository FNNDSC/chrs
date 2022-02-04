use std::path::PathBuf;
use crate::ChrisClient;


pub fn upload(client: &ChrisClient, files: &Vec<PathBuf>, path: &String) {
    let prefix = PathBuf::from(path);
    for file in files {
        let upload_path = prefix.join(file).to_string_lossy().into_owned();
        let url = client.upload(file, &upload_path);
        println!("{}", url);
    }
}
