use std::future::Future;

use crate::executor::{ComputePool, DeliverQueue, IoExecutor, UiQueue};
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

    pub fn spawn_ui(&mut self, task: Box<dyn FnOnce() + Send>) {
        self.ui_queue.push(task);
    }

    pub fn spawn_compute<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.compute_pool.spawn(task);
    }

    pub fn spawn_compute_with_class<F>(&self, class: PowerClass, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if self.power_policy.should_run(class) {
            self.compute_pool.spawn(task);
        }
    }

    pub fn spawn_io<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.io_executor.spawn(future);
    }

    pub fn tick(&mut self) {
        self.frame_clock.tick();
    }

    pub fn flush(&mut self) {
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
