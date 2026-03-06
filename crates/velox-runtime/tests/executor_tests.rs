use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use velox_runtime::executor::{ComputePool, DeliverQueue, IoExecutor, UiQueue};

#[test]
fn ui_queue_push_and_flush() {
    let counter = Arc::new(AtomicU32::new(0));
    let mut queue = UiQueue::new();

    let c = counter.clone();
    queue.push(Box::new(move || {
        c.fetch_add(1, Ordering::Relaxed);
    }));
    let c = counter.clone();
    queue.push(Box::new(move || {
        c.fetch_add(10, Ordering::Relaxed);
    }));

    assert_eq!(counter.load(Ordering::Relaxed), 0);
    queue.flush();
    assert_eq!(counter.load(Ordering::Relaxed), 11);
}

#[test]
fn ui_queue_flush_clears_queue() {
    let counter = Arc::new(AtomicU32::new(0));
    let mut queue = UiQueue::new();

    let c = counter.clone();
    queue.push(Box::new(move || {
        c.fetch_add(1, Ordering::Relaxed);
    }));

    queue.flush();
    queue.flush();
    assert_eq!(counter.load(Ordering::Relaxed), 1);
}

#[test]
fn ui_queue_is_empty() {
    let mut queue = UiQueue::new();
    assert!(queue.is_empty());
    queue.push(Box::new(|| {}));
    assert!(!queue.is_empty());
    queue.flush();
    assert!(queue.is_empty());
}

#[test]
fn compute_pool_executes_tasks() {
    let pool = ComputePool::new(2);
    let counter = Arc::new(AtomicU32::new(0));

    for _ in 0..10 {
        let c = counter.clone();
        pool.submit(Box::new(move || {
            c.fetch_add(1, Ordering::Relaxed);
        }));
    }

    drop(pool);
    assert_eq!(counter.load(Ordering::Relaxed), 10);
}

#[test]
fn compute_pool_bounded_threads() {
    let pool = ComputePool::new(4);
    let counter = Arc::new(AtomicU32::new(0));

    for _ in 0..100 {
        let c = counter.clone();
        pool.submit(Box::new(move || {
            c.fetch_add(1, Ordering::Relaxed);
        }));
    }

    drop(pool);
    assert_eq!(counter.load(Ordering::Relaxed), 100);
}

#[test]
fn io_executor_runs_async_task() {
    let executor = IoExecutor::new().expect("failed to create IoExecutor");
    let result = Arc::new(AtomicU32::new(0));

    let r = result.clone();
    executor.spawn(async move {
        r.store(42, Ordering::Relaxed);
    });

    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(result.load(Ordering::Relaxed), 42);
}

#[test]
fn io_executor_runs_multiple_tasks() {
    let executor = IoExecutor::new().expect("failed to create IoExecutor");
    let counter = Arc::new(AtomicU32::new(0));

    for _ in 0..10 {
        let c = counter.clone();
        executor.spawn(async move {
            c.fetch_add(1, Ordering::Relaxed);
        });
    }

    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(counter.load(Ordering::Relaxed), 10);
}

#[test]
fn deliver_queue_push_and_collect() {
    let mut queue = DeliverQueue::new();
    queue.push(1_u64, Box::new(42_i32));
    queue.push(2_u64, Box::new("hello".to_string()));

    let results = queue.drain();
    assert_eq!(results.len(), 2);
}

#[test]
fn deliver_queue_type_recovery() {
    let mut queue = DeliverQueue::new();
    queue.push(1_u64, Box::new(99_i32));

    let results = queue.drain();
    assert_eq!(results.len(), 1);

    let (task_id, value) = &results[0];
    assert_eq!(*task_id, 1);
    let recovered = value.downcast_ref::<i32>().expect("type mismatch");
    assert_eq!(*recovered, 99);
}

#[test]
fn deliver_queue_drain_clears() {
    let mut queue = DeliverQueue::new();
    queue.push(1_u64, Box::new(42_i32));

    let first = queue.drain();
    assert_eq!(first.len(), 1);

    let second = queue.drain();
    assert!(second.is_empty());
}

#[test]
fn deliver_queue_cross_thread() {
    let queue = Arc::new(std::sync::Mutex::new(DeliverQueue::new()));

    let q = queue.clone();
    let handle = std::thread::spawn(move || {
        q.lock().unwrap().push(1_u64, Box::new(42_i32));
    });

    handle.join().unwrap();
    let results = queue.lock().unwrap().drain();
    assert_eq!(results.len(), 1);
    let recovered = results[0].1.downcast_ref::<i32>().unwrap();
    assert_eq!(*recovered, 42);
}
