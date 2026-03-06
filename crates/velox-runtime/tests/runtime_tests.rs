use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use velox_runtime::{PowerClass, PowerPolicy, Runtime};

fn wait_for_delivery<T>(runtime: &mut Runtime, rx: &mpsc::Receiver<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        runtime.flush();
        match rx.try_recv() {
            Ok(value) => return value,
            Err(mpsc::TryRecvError::Disconnected) => panic!("delivery channel disconnected"),
            Err(mpsc::TryRecvError::Empty) => {
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for task delivery"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

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
    assert_eq!(wait_for_delivery(&mut runtime, &rx), 42);
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
    assert_eq!(wait_for_delivery(&mut runtime, &rx), 99);
}

#[test]
fn runtime_delivery_survives_late_callback_registration() {
    let mut runtime = Runtime::new();
    let task_id = runtime.spawn_compute(move || 11i32);

    std::thread::sleep(Duration::from_millis(50));
    runtime.flush();

    let (tx, rx) = mpsc::channel();
    runtime.register_deliver(task_id, move |result| {
        let value = *result.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });

    assert_eq!(wait_for_delivery(&mut runtime, &rx), 11);
}

#[test]
fn runtime_register_deliver_allows_non_send_ui_state() {
    let mut runtime = Runtime::new();
    let captured = Rc::new(Cell::new(0));
    let ui_state = Rc::clone(&captured);

    let task_id = runtime.spawn_compute(move || 7i32);
    runtime.register_deliver(task_id, move |result| {
        let value = *result.downcast::<i32>().unwrap();
        ui_state.set(value);
    });

    let deadline = Instant::now() + Duration::from_secs(2);
    while captured.get() == 0 {
        runtime.flush();
        assert!(
            Instant::now() < deadline,
            "timed out waiting for non-Send callback delivery"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(captured.get(), 7);
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
