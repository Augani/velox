use std::sync::mpsc;
use std::thread;

pub struct ComputePool {
    sender: Option<mpsc::Sender<Box<dyn FnOnce() + Send>>>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl ComputePool {
    pub fn new(num_threads: usize) -> Self {
        assert!(num_threads > 0, "ComputePool requires at least 1 thread");

        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
        let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));

        let mut workers = Vec::with_capacity(num_threads);
        for _ in 0..num_threads {
            let rx = receiver.clone();
            workers.push(thread::spawn(move || loop {
                let task = {
                    let lock = rx.lock().unwrap();
                    lock.recv()
                };
                match task {
                    Ok(task) => task(),
                    Err(_) => break,
                }
            }));
        }

        Self {
            sender: Some(sender),
            workers,
        }
    }

    pub fn submit(&self, task: Box<dyn FnOnce() + Send>) {
        if let Some(sender) = &self.sender {
            sender.send(task).expect("ComputePool has been shut down");
        }
    }
}

impl Drop for ComputePool {
    fn drop(&mut self) {
        self.sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
