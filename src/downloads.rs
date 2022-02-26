use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use chrono::{DateTime, Utc};
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
    pub pubdate: Option<DateTime<Utc>>,
    pub file_path: Option<PathBuf>,
}

/// This is the function the main controller uses to indicate new
/// files to download. It uses the threadpool to start jobs
/// for every episode to be downloaded. New jobs can be requested
/// by the user while there are still ongoing jobs.
pub fn download_list(
    episodes: Vec<EpData>,
    dest: &Path,
    max_retries: usize,
    threadpool: &Threadpool,
    tx_to_main: Sender<Message>,
) {
    // parse episode details and push to queue
    for ep in episodes.into_iter() {
        let tx = tx_to_main.clone();
        let dest2 = dest.to_path_buf();
        threadpool.execute(move || {
            let result = download_file(ep, dest2, max_retries);
            tx.send(Message::Dl(result))
                .expect("Thread messaging error");
        });
    }
}


/// Downloads a file to a local filepath, returning DownloadMsg variant
/// indicating success or failure.
fn download_file(mut ep_data: EpData, dest: PathBuf, mut max_retries: usize) -> DownloadMsg {
    let agent_builder = ureq::builder();
    #[cfg(feature = "native_tls")]
    let tls_connector = std::sync::Arc::new(native_tls::TlsConnector::new().unwrap());
    #[cfg(feature = "native_tls")]
    let agent_builder = agent_builder.tls_connector(tls_connector);
    let agent = agent_builder.build();

    let request: Result<ureq::Response, ()> = loop {
        let response = agent
            .get(&ep_data.url)
            .timeout(Duration::from_secs(30))
            .call();
        match response {
            Ok(resp) => break Ok(resp),
            Err(_) => {
                max_retries -= 1;
                if max_retries == 0 {
                    break Err(());
                }
            }
        }
    };

    if request.is_err() {
        return DownloadMsg::ResponseError(ep_data);
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

    let mut file_name = sanitize_with_options(&ep_data.title, Options {
        truncate: true,
        windows: true, // for simplicity, we'll just use Windows-friendly paths for everyone
        replacement: "",
    });

    if let Some(pubdate) = ep_data.pubdate {
        file_name = format!("{}_{}", file_name, pubdate.format("%Y%m%d_%H%M%S"));
    }

    let mut file_path = dest;
    file_path.push(format!("{file_name}.{ext}"));

    let dst = File::create(&file_path);
    if dst.is_err() {
        return DownloadMsg::FileCreateError(ep_data);
    };

    ep_data.file_path = Some(file_path);

    let mut reader = response.into_reader();
    return match std::io::copy(&mut reader, &mut dst.unwrap()) {
        Ok(_) => DownloadMsg::Complete(ep_data),
        Err(_) => DownloadMsg::FileWriteError(ep_data),
    };
}
