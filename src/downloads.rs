use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use sanitize_filename::{sanitize_with_options, Options};

use crate::threadpool::Threadpool;
use crate::types::Message;

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
pub fn download_list(
    episodes: Vec<EpData>,
    dest: &PathBuf,
    max_retries: usize,
    threadpool: &Threadpool,
    tx_to_main: Sender<Message>,
)
{
    // parse episode details and push to queue
    for ep in episodes.into_iter() {
        let tx = tx_to_main.clone();
        let dest2 = dest.clone();
        threadpool.execute(move || {
            let result = download_file(ep, dest2, max_retries);
            tx.send(Message::Dl(result)).unwrap();
        });
    }
}

/// Downloads a file to a local filepath, returning DownloadMsg variant
/// indicating success or failure.
fn download_file(ep_data: EpData, dest: PathBuf, mut max_retries: usize) -> DownloadMsg {
    let mut data = ep_data.clone();
    let request: Result<ureq::Response, ()> = loop {
        let response = ureq::get(&ep_data.url)
            .timeout_connect(5000)
            .timeout_read(30000)
            .call();
        if response.error() {
            max_retries -= 1;
            if max_retries == 0 {
                break Err(());
            }
        } else {
            break Ok(response);
        }
    };

    if request.is_err() {
        return DownloadMsg::ResponseError(data);
    };

    let response = request.unwrap();

    // figure out the file type
    let ext = match response.header("content-type") {
        Some("audio/x-m4a") => "m4a",
        Some("audio/mpeg") => "mp3",
        Some("video/quicktime") => "mov",
        Some("video/mp4") => "mp4",
        Some("video/x-m4v") => "m4v",
        _ => "mp3", // assume .mp3 unless we figure out otherwise
    };

    let file_name = sanitize_with_options(&ep_data.title, Options {
        truncate: true,
        windows: true, // for simplicity, we'll just use Windows-friendly paths for everyone
        replacement: "",
    });

    let mut file_path = dest;
    file_path.push(format!("{}.{}", file_name, ext));

    let dst = File::create(&file_path);
    if dst.is_err() {
        return DownloadMsg::FileCreateError(data);
    };

    data.file_path = Some(file_path);

    let mut reader = response.into_reader();
    return match std::io::copy(&mut reader, &mut dst.unwrap()) {
        Ok(_) => DownloadMsg::Complete(data),
        Err(_) => DownloadMsg::FileWriteError(data),
    };
}
