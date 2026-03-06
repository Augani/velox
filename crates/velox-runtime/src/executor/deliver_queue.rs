use std::any::Any;

pub struct DeliverQueue {
    entries: Vec<(u64, Box<dyn Any + Send>)>,
}

impl DeliverQueue {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, task_id: u64, value: Box<dyn Any + Send>) {
        self.entries.push((task_id, value));
    }

    pub fn drain(&mut self) -> Vec<(u64, Box<dyn Any + Send>)> {
        std::mem::take(&mut self.entries)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for DeliverQueue {
    fn default() -> Self {
        Self::new()
    }
}
