use std::cell::Cell;
use std::rc::Rc;
use velox_reactive::{Signal, batch};

#[test]
fn batch_defers_notifications() {
    let signal = Signal::new(0);
    let notify_count = Rc::new(Cell::new(0));
    let notify_count_clone = notify_count.clone();
    let _sub = signal.subscribe(move |_| {
        notify_count_clone.set(notify_count_clone.get() + 1);
    });

    batch(|| {
        signal.set(1);
        signal.set(2);
        signal.set(3);
        assert_eq!(notify_count.get(), 0);
    });
    assert_eq!(notify_count.get(), 1);
    assert_eq!(signal.get(), 3);
}

#[test]
fn batch_notifies_only_changed_signals() {
    let signal_a = Signal::new(0);
    let signal_b = Signal::new(0);
    let count_a = Rc::new(Cell::new(0));
    let count_b = Rc::new(Cell::new(0));
    let count_a_clone = count_a.clone();
    let count_b_clone = count_b.clone();
    let _sub_a = signal_a.subscribe(move |_| {
        count_a_clone.set(count_a_clone.get() + 1);
    });
    let _sub_b = signal_b.subscribe(move |_| {
        count_b_clone.set(count_b_clone.get() + 1);
    });

    batch(|| {
        signal_a.set(1);
    });
    assert_eq!(count_a.get(), 1);
    assert_eq!(count_b.get(), 0);
}

#[test]
fn batch_nested_only_flushes_on_outermost() {
    let signal = Signal::new(0);
    let notify_count = Rc::new(Cell::new(0));
    let notify_count_clone = notify_count.clone();
    let _sub = signal.subscribe(move |_| {
        notify_count_clone.set(notify_count_clone.get() + 1);
    });

    batch(|| {
        signal.set(1);
        batch(|| {
            signal.set(2);
        });
        assert_eq!(notify_count.get(), 0);
        signal.set(3);
    });
    assert_eq!(notify_count.get(), 1);
    assert_eq!(signal.get(), 3);
}

#[test]
fn without_batch_notifies_immediately() {
    let signal = Signal::new(0);
    let notify_count = Rc::new(Cell::new(0));
    let notify_count_clone = notify_count.clone();
    let _sub = signal.subscribe(move |_| {
        notify_count_clone.set(notify_count_clone.get() + 1);
    });
    signal.set(1);
    assert_eq!(notify_count.get(), 1);
    signal.set(2);
    assert_eq!(notify_count.get(), 2);
}

#[test]
fn batch_with_update() {
    let signal = Signal::new(vec![1]);
    let notify_count = Rc::new(Cell::new(0));
    let notify_count_clone = notify_count.clone();
    let _sub = signal.subscribe(move |_| {
        notify_count_clone.set(notify_count_clone.get() + 1);
    });

    batch(|| {
        signal.update(|v| v.push(2));
        signal.update(|v| v.push(3));
        assert_eq!(notify_count.get(), 0);
    });
    assert_eq!(notify_count.get(), 1);
    assert_eq!(signal.get(), vec![1, 2, 3]);
}
