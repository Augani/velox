use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc;

pub type TaskId = u64;

type TaskResult = Box<dyn Any + Send>;
type Callback = Box<dyn FnOnce(TaskResult)>;
type TaskEnvelope = (TaskId, TaskResult);

pub struct DeliverQueue {
    next_id: TaskId,
    callbacks: HashMap<TaskId, Callback>,
    pending_results: HashMap<TaskId, TaskResult>,
    sender: mpsc::Sender<TaskEnvelope>,
    receiver: mpsc::Receiver<TaskEnvelope>,
}

impl DeliverQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            next_id: 0,
            callbacks: HashMap::new(),
            pending_results: HashMap::new(),
            sender,
            receiver,
        }
    }

    pub fn register<F>(&mut self, callback: F) -> TaskId
    where
        F: FnOnce(TaskResult) + 'static,
    {
        let id = self.next_id;
        self.next_id += 1;
        self.register_for(id, callback);
        id
    }

    pub fn register_placeholder(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn register_for<F>(&mut self, task_id: TaskId, callback: F)
    where
        F: FnOnce(TaskResult) + 'static,
    {
        if let Some(result) = self.pending_results.remove(&task_id) {
            callback(result);
            return;
        }
        self.callbacks.insert(task_id, Box::new(callback));
    }

    pub fn sender(&self) -> mpsc::Sender<TaskEnvelope> {
        self.sender.clone()
    }

    pub fn send_result(&self, task_id: TaskId, result: TaskResult) {
        let _ = self.sender.send((task_id, result));
    }

    pub fn flush(&mut self) {
        while let Ok((task_id, result)) = self.receiver.try_recv() {
            if let Some(callback) = self.callbacks.remove(&task_id) {
                callback(result);
            } else {
                self.pending_results.insert(task_id, result);
            }
        }
    }
}

impl Default for DeliverQueue {
    fn default() -> Self {
        Self::new()
    }
}
