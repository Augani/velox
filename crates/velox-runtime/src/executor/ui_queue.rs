use std::collections::VecDeque;
use std::sync::mpsc;

pub struct UiQueue {
    tasks: VecDeque<Box<dyn FnOnce()>>,
    deferred_tx: mpsc::Sender<Box<dyn FnOnce() + Send>>,
    deferred_rx: mpsc::Receiver<Box<dyn FnOnce() + Send>>,
}

impl UiQueue {
    pub fn new() -> Self {
        let (deferred_tx, deferred_rx) = mpsc::channel();
        Self {
            tasks: VecDeque::new(),
            deferred_tx,
            deferred_rx,
        }
    }

    pub fn push(&mut self, task: Box<dyn FnOnce()>) {
        self.tasks.push_back(task);
    }

    pub fn flush(&mut self) {
        while let Some(task) = self.tasks.pop_front() {
            task();
        }
        while let Ok(task) = self.deferred_rx.try_recv() {
            task();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn deferred_sender(&self) -> mpsc::Sender<Box<dyn FnOnce() + Send>> {
        self.deferred_tx.clone()
    }
}

impl Default for UiQueue {
    fn default() -> Self {
        Self::new()
    }
}
