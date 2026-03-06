use std::any::Any;
use std::future::Future;

use crate::executor::{ComputePool, DeliverQueue, IoExecutor, TaskId, UiQueue};
use crate::frame_clock::FrameClock;
use crate::power::{PowerClass, PowerPolicy};

pub struct Runtime {
    ui_queue: UiQueue,
    compute_pool: ComputePool,
    io_executor: IoExecutor,
    deliver_queue: DeliverQueue,
    frame_clock: FrameClock,
    power_policy: PowerPolicy,
}

impl Runtime {
    pub fn new() -> Self {
        RuntimeBuilder::default().build()
    }

    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::default()
    }

    pub fn spawn_ui(&mut self, task: impl FnOnce() + 'static) {
        self.ui_queue.push(Box::new(task));
    }

    pub fn spawn_compute<T, F>(&mut self, work: F) -> TaskId
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();
        self.compute_pool.spawn(move || {
            let result = work();
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });
        task_id
    }

    pub fn spawn_compute_with_class<T, F>(&mut self, class: PowerClass, work: F) -> Option<TaskId>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        if !self.power_policy.should_run(class) {
            return None;
        }
        Some(self.spawn_compute(work))
    }

    pub fn spawn_io<T, F>(&mut self, future: F) -> TaskId
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();
        self.io_executor.spawn(async move {
            let result = future.await;
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });
        task_id
    }

    pub fn register_deliver<F>(&mut self, task_id: TaskId, callback: F)
    where
        F: FnOnce(Box<dyn Any + Send>) + 'static,
    {
        self.deliver_queue.register_for(task_id, callback);
    }

    pub fn tick(&mut self) {
        self.frame_clock.tick();
    }

    pub fn flush(&mut self) {
        self.frame_clock.tick();
        self.ui_queue.flush();
        self.deliver_queue.flush();
    }

    pub fn frame_clock(&self) -> &FrameClock {
        &self.frame_clock
    }

    pub fn deliver_queue(&mut self) -> &mut DeliverQueue {
        &mut self.deliver_queue
    }

    pub fn power_policy(&self) -> PowerPolicy {
        self.power_policy
    }

    pub fn set_power_policy(&mut self, policy: PowerPolicy) {
        self.power_policy = policy;
    }

    pub fn with_power_policy(mut self, policy: PowerPolicy) -> Self {
        self.power_policy = policy;
        self
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RuntimeBuilder {
    compute_threads: usize,
    power_policy: PowerPolicy,
}

impl RuntimeBuilder {
    pub fn compute_threads(mut self, n: usize) -> Self {
        self.compute_threads = n;
        self
    }

    pub fn power_policy(mut self, policy: PowerPolicy) -> Self {
        self.power_policy = policy;
        self
    }

    pub fn build(self) -> Runtime {
        Runtime {
            ui_queue: UiQueue::new(),
            compute_pool: ComputePool::new(self.compute_threads),
            io_executor: IoExecutor::new(),
            deliver_queue: DeliverQueue::new(),
            frame_clock: FrameClock::new(),
            power_policy: self.power_policy,
        }
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self {
            compute_threads: num_cpus(),
            power_policy: PowerPolicy::default(),
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
}
