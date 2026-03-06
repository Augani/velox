use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use velox::prelude::*;
use velox::runtime::Runtime;

fn wait_for<T>(runtime: &mut Runtime, rx: &mpsc::Receiver<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        runtime.flush();
        match rx.try_recv() {
            Ok(value) => return value,
            Err(mpsc::TryRecvError::Disconnected) => panic!("delivery channel disconnected"),
            Err(mpsc::TryRecvError::Empty) => {
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for runtime delivery"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn main() {
    let count = Signal::new(1);
    let doubled = Computed::new({
        let count = count.clone();
        move || count.get() * 2
    });
    assert_eq!(doubled.get(), 2);

    let signal_notifications = Rc::new(Cell::new(0));
    let signal_notifications_clone = signal_notifications.clone();
    let _signal_sub = count.subscribe(move |_| {
        signal_notifications_clone.set(signal_notifications_clone.get() + 1);
    });

    Batch::run(|| {
        count.set(2);
        count.set(3);
    });

    assert_eq!(count.get(), 3);
    assert_eq!(doubled.get(), 6);
    assert_eq!(signal_notifications.get(), 1);

    let event: Event<&'static str> = Event::new();
    let event_seen = Rc::new(Cell::new(false));
    let event_seen_clone = event_seen.clone();
    let _event_sub = event.subscribe(move |message| {
        if *message == "ready" {
            event_seen_clone.set(true);
        }
    });
    event.emit("ready");
    assert!(event_seen.get());

    let mut runtime = Runtime::new();

    let (ui_tx, ui_rx) = mpsc::channel();
    runtime.spawn_ui(move || {
        ui_tx.send("ui-ok").expect("failed to send UI marker");
    });
    runtime.flush();
    assert_eq!(
        ui_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("failed to receive UI marker"),
        "ui-ok"
    );

    let (compute_tx, compute_rx) = mpsc::channel();
    let compute_task = runtime.spawn_compute(|| 21 * 2);
    runtime.register_deliver(compute_task, move |result| {
        let value = *result
            .downcast::<i32>()
            .expect("compute result should be i32");
        compute_tx
            .send(value)
            .expect("failed to send compute result");
    });
    assert_eq!(wait_for(&mut runtime, &compute_rx), 42);

    let (io_tx, io_rx) = mpsc::channel();
    let io_task = runtime.spawn_io(async { String::from("io-ok") });
    runtime.register_deliver(io_task, move |result| {
        let value = *result
            .downcast::<String>()
            .expect("io result should be String");
        io_tx.send(value).expect("failed to send io result");
    });
    assert_eq!(wait_for(&mut runtime, &io_rx), "io-ok");

    runtime.set_power_policy(PowerPolicy::Saving);
    let decorative_task = runtime.spawn_compute_with_class(PowerClass::Decorative, || 1usize);
    assert!(decorative_task.is_none());

    println!("phase1_smoke completed successfully");
}
