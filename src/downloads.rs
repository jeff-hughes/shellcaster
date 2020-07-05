use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use sanitize_filename::{sanitize_with_options, Options};

use crate::types::Message;
use crate::threadpool::Threadpool;

/// Enum used for communicating back to the main controller upon
/// successful or unsuccessful downloading of a file. i32 value
/// represents the episode ID, and PathBuf the location of the new file.
#[derive(Debug)]
pub enum DownloadMsg {
    Complete(EpData),
    ResponseError(EpData),
    FileCreateError(EpData),
    FileWriteError(EpData),
}

/// Enum used to communicate relevant data to the threadpool.
#[derive(Debug, Clone)]
pub struct EpData {
    pub id: i64,
    pub pod_id: i64,
    pub title: String,
    pub url: String,
    pub file_path: Option<PathBuf>,
}

/// This is the function the main controller uses to indicate new
/// files to download. It uses the threadpool to start jobs
/// for every episode to be downloaded. New jobs can be requested
/// by the user while there are still ongoing jobs.
pub fn download_list(episodes: Vec<EpData>, dest: &PathBuf, threadpool: &Threadpool, tx_to_main: Sender<Message>) {
    // parse episode details and push to queue
    for mut ep in episodes.into_iter() {
        let file_name = sanitize_with_options(&ep.title, Options {
            truncate: true,
            windows: true,  // for simplicity, we'll just use Windows-friendly paths for everyone
            replacement: ""
        });

        let mut file_path = dest.clone();
        file_path.push(format!("{}.mp3", file_name));

        ep.file_path = Some(file_path);

        let tx = tx_to_main.clone();
        threadpool.execute(move || {
            let result = download_file(ep);
            tx.send(Message::Dl(result)).unwrap();
        });
    }
}


/// Downloads a file to a local filepath, returning DownloadMsg variant
/// indicating success or failure.
fn download_file(ep_data: EpData) -> DownloadMsg {
    let data = ep_data.clone();

    let dst = File::create(&ep_data.file_path.unwrap());
    if dst.is_err() {
        return DownloadMsg::FileCreateError(data);
    };

    let response = ureq::get(&ep_data.url)
        .timeout_connect(5000)
        .timeout_read(30000)
        .call();
    if response.error() {
        return DownloadMsg::ResponseError(data);
    }

    let mut reader = response.into_reader();
    return match std::io::copy(&mut reader, &mut dst.unwrap()) {
        Ok(_) => DownloadMsg::Complete(data),
        Err(_) => DownloadMsg::FileWriteError(data),
    };
}