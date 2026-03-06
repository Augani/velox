use std::sync::mpsc;
use std::time::Duration;
use velox_runtime::{PowerClass, PowerPolicy, Runtime};

#[test]
fn runtime_spawn_ui_runs_on_flush() {
    let mut runtime = Runtime::new();
    let (tx, rx) = mpsc::channel();
    runtime.spawn_ui(move || {
        tx.send(42).unwrap();
    });
    runtime.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 42);
}

#[test]
fn runtime_spawn_compute_delivers_result() {
    let mut runtime = Runtime::new();
    let (tx, rx) = mpsc::channel();
    let task_id = runtime.spawn_compute(move || 42i32);
    runtime.register_deliver(task_id, move |result| {
        let value = *result.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });
    std::thread::sleep(Duration::from_millis(100));
    runtime.flush();
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), 42);
}

#[test]
fn runtime_spawn_io_delivers_result() {
    let mut runtime = Runtime::new();
    let (tx, rx) = mpsc::channel();
    let task_id = runtime.spawn_io(async { 99i32 });
    runtime.register_deliver(task_id, move |result| {
        let value = *result.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });
    std::thread::sleep(Duration::from_millis(100));
    runtime.flush();
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), 99);
}

#[test]
fn power_policy_gates_decorative_on_saving() {
    let policy = PowerPolicy::Saving;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(!policy.should_run(PowerClass::Decorative));
    assert!(!policy.should_run(PowerClass::Background));
}

#[test]
fn power_policy_adaptive_allows_all() {
    let policy = PowerPolicy::Adaptive;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(policy.should_run(PowerClass::Decorative));
    assert!(policy.should_run(PowerClass::Background));
}

#[test]
fn power_policy_performance_allows_all() {
    let policy = PowerPolicy::Performance;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Decorative));
    assert!(policy.should_run(PowerClass::Background));
}
