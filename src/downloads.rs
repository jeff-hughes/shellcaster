use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc, mpsc::Sender};
use std::thread;

use reqwest::blocking::Client;

use crate::types::{Episode, Message};

/// Enum used for communicating back to the main controller upon
/// successful or unsuccessful downloading of a file. i32 value
/// represents the episode ID, and PathBuf the location of the new file.
#[derive(Debug)]
pub enum DownloadMsg {
    Complete(EpData),
    ResponseError(EpData),
    ResponseDataError(EpData),
    FileCreateError(EpData),
    FileWriteError(EpData),
}

/// Enum used to communicate relevant data to the threadpool.
#[derive(Debug, Clone)]
pub struct EpData {
    pub id: i64,
    pub pod_id: i64,
    pub url: String,
    pub file_path: PathBuf,
}


/// Main controller for managing downloads. The DownloadManager will
/// spin up a threadpool the first time downloads are requested.
pub struct DownloadManager {
    request_client: Client,
    tx_to_main: Sender<Message>,
    threadpool: Option<ThreadPool>,
    n_threads: usize,
}

impl DownloadManager {
    /// Creates a new DownloadManager.
    pub fn new(n_threads: usize, tx_to_main: Sender<Message>) -> DownloadManager {
        return DownloadManager {
            request_client: Client::new(),
            tx_to_main: tx_to_main,
            threadpool: None,
            n_threads: n_threads,
        };
    }

    /// Clones the reference to the reqwest client.
    fn get_client(&self) -> Client {
        return self.request_client.clone();
    }

    /// This is the method the main controller uses to indicate new
    /// files to download. The DownloadManager will spin up a new
    /// threadpool if one is not already running, and then starts jobs
    /// for every episode to be downloaded. New jobs can be requested
    /// by the user while there are still ongoing jobs.
    pub fn download_list(&mut self, episodes: &[&Episode], dest: &PathBuf) {
        // download thread and threadpool only exist when download
        // queue is not empty
        if self.threadpool.is_none() {
            self.threadpool = Some(ThreadPool::new(self.n_threads));
        }

        // parse episode details and push to queue
        for ep in episodes.iter() {
            let mut file_path = dest.clone();
            file_path.push(format!("{}.mp3", ep.title));

            let ep_data = EpData {
                id: ep.id.unwrap(),
                pod_id: ep.pod_id.unwrap(),
                url: ep.url.clone(),
                file_path: file_path,
            };

            let client_clone = self.get_client();
            let tx_to_main = self.tx_to_main.clone();

            self.threadpool.as_ref().unwrap().execute(move || {
                let result = download_file(client_clone, ep_data);
                tx_to_main.send(Message::Dl(result)).unwrap();
            });
        }
    }
}


/// Downloads a file to a local filepath, returning DownloadMsg variant
/// indicating success or failure.
fn download_file(client: Client, ep_data: EpData) -> DownloadMsg {
    let response = client.get(&ep_data.url).send();
    if response.is_err() {
        return DownloadMsg::ResponseError(ep_data);
    }

    let resp_data = response.unwrap().bytes();
    if resp_data.is_err() {
        return DownloadMsg::ResponseDataError(ep_data);
    }

    let dst = File::create(&ep_data.file_path);
    if dst.is_err() {
        return DownloadMsg::FileCreateError(ep_data);
    };

    return match dst.unwrap().write(&resp_data.unwrap()) {
        Ok(_) => DownloadMsg::Complete(ep_data),
        Err(_) => DownloadMsg::FileWriteError(ep_data),
    };
}


// Much of the threadpool implementation here was taken directly from
// the Rust Book: https://doc.rust-lang.org/book/ch20-02-multithreaded.html
// and https://doc.rust-lang.org/book/ch20-03-graceful-shutdown-and-cleanup.html

/// Manages a threadpool of a given size, sending jobs to workers as
/// necessary. Implements Drop trait to allow threads to complete 
/// their current jobs before being stopped.
struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<JobMessage>,
}

impl ThreadPool {
    /// Creates a new ThreadPool of a given size.
    fn new(n_threads: usize) -> ThreadPool {
        let (sender, receiver) = mpsc::channel();
        let receiver_lock = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(n_threads);

        for _ in 0..n_threads {
            workers.push(Worker::new(Arc::clone(&receiver_lock)));
        }

        return ThreadPool {
            workers: workers,
            sender: sender,
        };
    }

    /// Adds a new job to the threadpool, passing closure to first
    /// available worker.
    fn execute<F>(&self, func: F)
        where F: FnOnce() + Send + 'static {

        let job = Box::new(func);
        self.sender.send(JobMessage::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    /// Upon going out of scope, ThreadPool sends terminate message to
    /// all workers but allows them to complete current jobs.
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(JobMessage::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                // joins to ensure threads finish job before stopping
                thread.join().unwrap();
            }
        }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

/// Messages used by ThreadPool to communicate with Workers.
enum JobMessage {
    NewJob(Job),
    Terminate,
}

/// Used by ThreadPool to complete jobs. Each Worker manages a single
/// thread.
struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Creates a new Worker, which waits for Jobs to be passed by the
    /// ThreadPool.
    fn new(receiver: Arc<Mutex<mpsc::Receiver<JobMessage>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                JobMessage::NewJob(job) => job(),
                JobMessage::Terminate => break,
            }
        });

        return Worker {
            thread: Some(thread),
        };
    }
}