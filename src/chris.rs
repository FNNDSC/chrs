use lazy_static::lazy_static;
use reqwest::blocking::multipart;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

lazy_static! {
    static ref CLIENT: reqwest::blocking::Client = reqwest::blocking::Client::new();
}

#[derive(Debug)]
pub struct ChrisClient {
    username: String,
    token: String,
    links: CUBELinks,
}

#[derive(Debug)]
struct CUBELinks {
    uploadedfiles: String,
    // user: String
}

#[derive(Deserialize)]
struct AuthTokenResponse {
    token: String,
}

#[derive(Deserialize)]
struct UploadedFilesResponse {
    url: String,
    // unused
    // id: u32,
    // creation_date: String,
    // fname: String,
    // fsize: u32,
    // file_resource: String,
    // owner: String
}

impl ChrisClient {
    pub fn new(address: &String, username: &String, password: &String) -> ChrisClient {
        if !address.starts_with("http") {
            panic!("address must start with http");
        }

        let login_uri = format!("{}auth-token/", address);

        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());

        let mut payload = HashMap::new();
        payload.insert("username", username);
        payload.insert("password", password);

        let req = CLIENT.post(login_uri).headers(headers).json(&payload);

        let res = req.send();

        // TODO exception handling, what if address is wrong?
        let token_object: AuthTokenResponse = res.unwrap().json().unwrap();

        ChrisClient {
            username: username.to_owned(),
            token: token_object.token,
            // address: address.to_owned(),
            links: CUBELinks {
                uploadedfiles: format!("{}uploadedfiles/", address),
                // user: format!("{}user/", address),
            },
        }
    }

    pub fn upload(&self, local_file: &Path, upload_path: &String) -> String {
        // TODO async
        let swift_path = format!("{}/uploads/{}", self.username, upload_path);

        let form = multipart::Form::new()
            .text("upload_path", swift_path.to_string())
            .file("fname", local_file)
            .unwrap();

        let req = CLIENT
            .post(&self.links.uploadedfiles)
            .header("accept", "application/json")
            .header("Authorization", format!("token {}", &self.token))
            .multipart(form);
        let res = req.send();
        let data: UploadedFilesResponse = res.unwrap().json().unwrap();
        data.url
    }
}
