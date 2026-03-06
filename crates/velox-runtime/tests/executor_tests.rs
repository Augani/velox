use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use velox_runtime::executor::{ComputePool, DeliverQueue, IoExecutor, UiQueue};

#[test]
fn ui_queue_runs_tasks_on_flush() {
    let mut queue = UiQueue::new();
    let (tx, rx) = mpsc::channel();
    queue.push(Box::new(move || {
        tx.send(42).unwrap();
    }));
    assert!(rx.try_recv().is_err());
    queue.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 42);
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
fn compute_pool_runs_work_on_background_thread() {
    let pool = ComputePool::new(2);
    let (tx, rx) = mpsc::channel();
    pool.spawn(move || {
        tx.send(std::thread::current().id()).unwrap();
    });
    let worker_thread = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_ne!(worker_thread, std::thread::current().id());
}

#[test]
fn compute_pool_handles_multiple_tasks() {
    let pool = ComputePool::new(2);
    let (tx, rx) = mpsc::channel();
    for i in 0..10 {
        let tx = tx.clone();
        pool.spawn(move || {
            tx.send(i).unwrap();
        });
    }
    let mut results: Vec<i32> =
        (0..10).map(|_| rx.recv_timeout(Duration::from_secs(2)).unwrap()).collect();
    results.sort();
    assert_eq!(results, (0..10).collect::<Vec<_>>());
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
fn io_executor_runs_async_work() {
    let executor = IoExecutor::new();
    let (tx, rx) = mpsc::channel();
    executor.spawn(async move {
        tx.send(42).unwrap();
    });
    let result = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn io_executor_runs_multiple_tasks() {
    let executor = IoExecutor::new();
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
fn deliver_queue_matches_results_to_callbacks() {
    let mut deliver = DeliverQueue::new();
    let (tx, rx) = mpsc::channel();
    let task_id = deliver.register(move |boxed: Box<dyn std::any::Any + Send>| {
        let value = *boxed.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });
    deliver.send_result(task_id, Box::new(99i32));
    deliver.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 99);
}

#[test]
fn deliver_queue_ignores_unregistered_results() {
    let mut deliver = DeliverQueue::new();
    deliver.send_result(999, Box::new(42i32));
    deliver.flush();
}

#[test]
fn deliver_queue_placeholder_then_register() {
    let mut deliver = DeliverQueue::new();
    let task_id = deliver.register_placeholder();

    let (tx, rx) = mpsc::channel();
    deliver.register_for(task_id, move |boxed: Box<dyn std::any::Any + Send>| {
        let value = *boxed.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });

    deliver.send_result(task_id, Box::new(77i32));
    deliver.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 77);
}

#[test]
fn deliver_queue_cross_thread_via_sender() {
    let mut deliver = DeliverQueue::new();
    let (tx, rx) = mpsc::channel();
    let task_id = deliver.register(move |boxed: Box<dyn std::any::Any + Send>| {
        let value = *boxed.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });

    let sender = deliver.sender();
    std::thread::spawn(move || {
        sender.send((task_id, Box::new(42i32) as Box<dyn std::any::Any + Send>)).unwrap();
    })
    .join()
    .unwrap();

    deliver.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 42);
}
