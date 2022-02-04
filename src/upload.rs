use crate::ChrisClient;


pub fn upload(client: &ChrisClient, files: &Vec<String>, path: &String) {
    let prefix = path2prefix(path);

    for file in files {
        let upload_path = format!("{}{}", prefix, file);
        let url = client.upload(file, &upload_path);
        println!("{}", url);
    }
}


fn path2prefix(path: &String) -> String {
    if path.is_empty() {
        return String::new()
    }
    format!("{}/", path)
}
