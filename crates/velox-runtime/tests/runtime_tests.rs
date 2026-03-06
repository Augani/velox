use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use velox_runtime::power::{PowerClass, PowerPolicy};
use velox_runtime::Runtime;

#[test]
fn runtime_creates_with_defaults() {
    let rt = Runtime::new();
    assert_eq!(rt.frame_clock().frame_count(), 0);
}

#[test]
fn runtime_spawn_ui_executes_on_flush() {
    let mut rt = Runtime::new();
    let (tx, rx) = mpsc::channel();
    rt.spawn_ui(Box::new(move || {
        tx.send(42).unwrap();
    }));
    assert!(rx.try_recv().is_err());
    rt.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 42);
}

#[test]
fn runtime_spawn_compute_runs_on_background() {
    let rt = Runtime::new();
    let (tx, rx) = mpsc::channel();
    rt.spawn_compute(move || {
        tx.send(std::thread::current().id()).unwrap();
    });
    let worker = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_ne!(worker, std::thread::current().id());
}

#[test]
fn runtime_spawn_io_runs_async() {
    let rt = Runtime::new();
    let (tx, rx) = mpsc::channel();
    rt.spawn_io(async move {
        tx.send(42).unwrap();
    });
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), 42);
}

#[test]
fn runtime_tick_advances_frame_clock() {
    let mut rt = Runtime::new();
    rt.tick();
    assert_eq!(rt.frame_clock().frame_count(), 1);
    rt.tick();
    assert_eq!(rt.frame_clock().frame_count(), 2);
}

#[test]
fn runtime_flush_drains_ui_and_deliver() {
    let mut rt = Runtime::new();
    let counter = Arc::new(AtomicU32::new(0));

    let c = counter.clone();
    rt.spawn_ui(Box::new(move || {
        c.fetch_add(1, Ordering::Relaxed);
    }));
    let c = counter.clone();
    rt.spawn_ui(Box::new(move || {
        c.fetch_add(10, Ordering::Relaxed);
    }));

    rt.flush();
    assert_eq!(counter.load(Ordering::Relaxed), 11);
}

#[test]
fn runtime_power_policy_gates_compute() {
    let rt = Runtime::builder()
        .power_policy(PowerPolicy::Saving)
        .build();

    let counter = Arc::new(AtomicU32::new(0));

    let c = counter.clone();
    rt.spawn_compute_with_class(PowerClass::Background, move || {
        c.fetch_add(1, Ordering::Relaxed);
    });

    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    let c = counter.clone();
    rt.spawn_compute_with_class(PowerClass::Essential, move || {
        c.fetch_add(10, Ordering::Relaxed);
    });

    drop(rt);
    assert_eq!(counter.load(Ordering::Relaxed), 10);
}

#[test]
fn runtime_set_power_policy() {
    let mut rt = Runtime::new();
    rt.set_power_policy(PowerPolicy::Saving);

    let counter = Arc::new(AtomicU32::new(0));
    let c = counter.clone();
    rt.spawn_compute_with_class(PowerClass::Decorative, move || {
        c.fetch_add(1, Ordering::Relaxed);
    });

    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}

#[test]
fn runtime_builder_custom_threads() {
    let rt = Runtime::builder()
        .compute_threads(4)
        .build();

    let (tx, rx) = mpsc::channel();
    for _ in 0..8 {
        let tx = tx.clone();
        rt.spawn_compute(move || {
            tx.send(1).unwrap();
        });
    }

    let total: i32 = (0..8)
        .map(|_| rx.recv_timeout(Duration::from_secs(2)).unwrap())
        .sum();
    assert_eq!(total, 8);
}
