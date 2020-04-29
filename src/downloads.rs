use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use reqwest::Client;
use futures::future::join_all;

use crate::types::Episode;

pub struct DownloadManager {
    client: Client,
}

impl DownloadManager {

    pub fn new() -> DownloadManager {
        return DownloadManager {
            client: Client::new(),
        };
    }

    fn get_client<'a>(&'a self) -> &'a Client {
        return &self.client;
    }

    #[tokio::main]
    pub async fn download_list<'a>(&self, episodes: &Vec<&Episode>, dest: &'a PathBuf) ->
    Vec<Result<Option<PathBuf>, Box<dyn std::error::Error>>> {

        let mut eps_vec = Vec::new();
        for ep in episodes {
            let mut file_path = dest.clone();
            file_path.push(format!("{}.mp3", ep.title));

            eps_vec.push(self.download_file(&ep.url, file_path));
        }
        let return_vec = join_all(eps_vec).await;
        return return_vec;
    }

    pub async fn download_file(&self, url: &str, file_path: PathBuf) ->
    Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        
        // Unfortunately, the use of join_all() in download_list()
        // means that none of the futures returned here can return an
        // error, or else join_all() returns early and cancels the other 
        // futures. So instead we explicitly throw away the errors; a
        // None value indicates that there was an error, but...I'm still
        // looking for a better solution to this.

        let client = self.get_client();
        let response = match client.get(url).send().await {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let resp_data = match response.bytes().await {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        let mut dst = match File::create(&file_path) {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };

        match dst.write(&resp_data) {
            Ok(_) => return Ok(Some(file_path)),
            Err(_) => return Ok(None),
        };
    }
}