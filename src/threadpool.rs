use std::sync::{Arc, Mutex, mpsc};
use std::thread;

// Much of the threadpool implementation here was taken directly from
// the Rust Book: https://doc.rust-lang.org/book/ch20-02-multithreaded.html
// and https://doc.rust-lang.org/book/ch20-03-graceful-shutdown-and-cleanup.html

/// Manages a threadpool of a given size, sending jobs to workers as
/// necessary. Implements Drop trait to allow threads to complete 
/// their current jobs before being stopped.
pub struct Threadpool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<JobMessage>,
}

impl Threadpool {
    /// Creates a new Threadpool of a given size.
    pub fn new(n_threads: usize) -> Threadpool {
        let (sender, receiver) = mpsc::channel();
        let receiver_lock = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(n_threads);

        for _ in 0..n_threads {
            workers.push(Worker::new(Arc::clone(&receiver_lock)));
        }

        return Threadpool {
            workers: workers,
            sender: sender,
        };
    }

    /// Adds a new job to the threadpool, passing closure to first
    /// available worker.
    pub fn execute<F>(&self, func: F)
        where F: FnOnce() + Send + 'static {

        let job = Box::new(func);
        self.sender.send(JobMessage::NewJob(job)).unwrap();
    }
}

impl Drop for Threadpool {
    /// Upon going out of scope, Threadpool sends terminate message to
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

/// Messages used by Threadpool to communicate with Workers.
enum JobMessage {
    NewJob(Job),
    Terminate,
}

/// Used by Threadpool to complete jobs. Each Worker manages a single
/// thread.
struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Creates a new Worker, which waits for Jobs to be passed by the
    /// Threadpool.
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