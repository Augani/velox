use std::collections::VecDeque;

pub struct UiQueue {
    tasks: VecDeque<Box<dyn FnOnce() + Send>>,
}

impl UiQueue {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    pub fn push(&mut self, task: Box<dyn FnOnce() + Send>) {
        self.tasks.push_back(task);
    }

    pub fn flush(&mut self) {
        while let Some(task) = self.tasks.pop_front() {
            task();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for UiQueue {
    fn default() -> Self {
        Self::new()
    }
}
