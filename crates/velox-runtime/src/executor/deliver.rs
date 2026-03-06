use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc;

pub type TaskId = u64;

type Callback = Box<dyn FnOnce(Box<dyn Any + Send>) + Send>;

pub struct DeliverQueue {
    next_id: TaskId,
    callbacks: HashMap<TaskId, Callback>,
    sender: mpsc::Sender<(TaskId, Box<dyn Any + Send>)>,
    receiver: mpsc::Receiver<(TaskId, Box<dyn Any + Send>)>,
}

impl DeliverQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            next_id: 0,
            callbacks: HashMap::new(),
            sender,
            receiver,
        }
    }

    pub fn register<F>(&mut self, callback: F) -> TaskId
    where
        F: FnOnce(Box<dyn Any + Send>) + Send + 'static,
    {
        let id = self.next_id;
        self.next_id += 1;
        self.callbacks.insert(id, Box::new(callback));
        id
    }

    pub fn register_placeholder(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn register_for<F>(&mut self, task_id: TaskId, callback: F)
    where
        F: FnOnce(Box<dyn Any + Send>) + Send + 'static,
    {
        self.callbacks.insert(task_id, Box::new(callback));
    }

    pub fn sender(&self) -> mpsc::Sender<(TaskId, Box<dyn Any + Send>)> {
        self.sender.clone()
    }

    pub fn send_result(&self, task_id: TaskId, result: Box<dyn Any + Send>) {
        let _ = self.sender.send((task_id, result));
    }

    pub fn flush(&mut self) {
        while let Ok((task_id, result)) = self.receiver.try_recv() {
            if let Some(callback) = self.callbacks.remove(&task_id) {
                callback(result);
            }
        }
    }
}

impl Default for DeliverQueue {
    fn default() -> Self {
        Self::new()
    }
}
