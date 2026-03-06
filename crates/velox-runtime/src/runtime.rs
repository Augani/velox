use std::any::Any;
use std::collections::VecDeque;
use std::future::Future;
use std::time::Duration;

use tracing::trace;

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
    idle_tasks: VecDeque<Box<dyn FnOnce()>>,
    shutdown: bool,
}

impl Runtime {
    pub fn new() -> Self {
        RuntimeBuilder::default().build()
    }

    pub fn builder() -> RuntimeBuilder {
        RuntimeBuilder::default()
    }

    pub fn spawn_ui(&mut self, task: impl FnOnce() + 'static) {
        if self.shutdown {
            return;
        }
        self.ui_queue.push(Box::new(task));
    }

    pub fn spawn_compute<T, F>(&mut self, work: F) -> TaskId
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        if self.shutdown {
            return 0;
        }
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
        if self.shutdown {
            return None;
        }
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
        if self.shutdown {
            return 0;
        }
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();
        self.io_executor.spawn(async move {
            let result = future.await;
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });
        task_id
    }

    pub fn spawn_after(&self, delay: Duration, task: impl FnOnce() + Send + 'static) {
        if self.shutdown {
            return;
        }
        let sender = self.ui_queue.deferred_sender();
        self.io_executor.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = sender.send(Box::new(task));
        });
    }

    pub fn spawn_idle(&mut self, task: impl FnOnce() + 'static) {
        if self.shutdown {
            return;
        }
        self.idle_tasks.push_back(Box::new(task));
    }

    pub fn spawn_ui_labeled(&mut self, label: &'static str, task: impl FnOnce() + 'static) {
        if self.shutdown {
            return;
        }
        trace!(task = label, "spawn_ui");
        self.ui_queue.push(Box::new(task));
    }

    pub fn spawn_compute_labeled<T, F>(&mut self, label: &'static str, work: F) -> TaskId
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        if self.shutdown {
            return 0;
        }
        trace!(task = label, "spawn_compute");
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();
        self.compute_pool.spawn(move || {
            let result = work();
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });
        task_id
    }

    pub fn spawn_io_labeled<T, F>(&mut self, label: &'static str, future: F) -> TaskId
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        if self.shutdown {
            return 0;
        }
        trace!(task = label, "spawn_io");
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();
        self.io_executor.spawn(async move {
            let result = future.await;
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });
        task_id
    }

    pub fn spawn_after_labeled(
        &self,
        label: &'static str,
        delay: Duration,
        task: impl FnOnce() + Send + 'static,
    ) {
        if self.shutdown {
            return;
        }
        trace!(task = label, ?delay, "spawn_after");
        let sender = self.ui_queue.deferred_sender();
        self.io_executor.spawn(async move {
            tokio::time::sleep(delay).await;
            let _ = sender.send(Box::new(task));
        });
    }

    pub fn spawn_idle_labeled(&mut self, label: &'static str, task: impl FnOnce() + 'static) {
        if self.shutdown {
            return;
        }
        trace!(task = label, "spawn_idle");
        self.idle_tasks.push_back(Box::new(task));
    }

    pub fn flush_idle(&mut self) {
        let tasks: VecDeque<Box<dyn FnOnce()>> = std::mem::take(&mut self.idle_tasks);
        for task in tasks {
            task();
        }
    }

    pub fn has_pending_idle(&self) -> bool {
        !self.idle_tasks.is_empty()
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

    pub fn shutdown(mut self) {
        self.shutdown = true;
        self.ui_queue.flush();
        self.deliver_queue.flush();
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
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
            idle_tasks: VecDeque::new(),
            shutdown: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn spawn_after_fires_callback() {
        let mut runtime = Runtime::new();
        let result = Arc::new(Mutex::new(None));
        let r = result.clone();
        runtime.spawn_after(Duration::from_millis(50), move || {
            *r.lock().unwrap() = Some(42);
        });
        std::thread::sleep(Duration::from_millis(150));
        runtime.flush();
        assert_eq!(*result.lock().unwrap(), Some(42));
    }

    #[test]
    fn spawn_idle_push_and_flush() {
        let mut runtime = Runtime::new();
        let result = Arc::new(Mutex::new(Vec::new()));
        let r1 = result.clone();
        let r2 = result.clone();

        runtime.spawn_idle(move || r1.lock().unwrap().push(1));
        runtime.spawn_idle(move || r2.lock().unwrap().push(2));

        assert!(runtime.has_pending_idle());
        runtime.flush_idle();
        assert!(!runtime.has_pending_idle());

        let vals = result.lock().unwrap();
        assert_eq!(*vals, vec![1, 2]);
    }

    #[test]
    fn has_pending_idle_empty() {
        let runtime = Runtime::new();
        assert!(!runtime.has_pending_idle());
    }

    #[test]
    fn shutdown_prevents_spawn_ui() {
        let mut runtime = Runtime::new();
        let result = Arc::new(Mutex::new(false));
        let r = result.clone();
        runtime.shutdown = true;
        runtime.spawn_ui(move || *r.lock().unwrap() = true);
        assert!(!*result.lock().unwrap());
    }

    #[test]
    fn shutdown_prevents_spawn_idle() {
        let mut runtime = Runtime::new();
        let result = Arc::new(Mutex::new(false));
        let r = result.clone();
        runtime.shutdown = true;
        runtime.spawn_idle(move || *r.lock().unwrap() = true);
        assert!(!runtime.has_pending_idle());
    }

    #[test]
    fn shutdown_prevents_spawn_compute() {
        let mut runtime = Runtime::new();
        runtime.shutdown = true;
        let task_id = runtime.spawn_compute(|| 42);
        assert_eq!(task_id, 0);
    }

    #[test]
    fn shutdown_method_drains_queues() {
        let mut runtime = Runtime::new();
        let result = Arc::new(Mutex::new(false));
        let r = result.clone();
        runtime.spawn_ui(move || *r.lock().unwrap() = true);
        runtime.shutdown();
        assert!(*result.lock().unwrap());
    }
}
