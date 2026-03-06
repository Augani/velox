use std::cell::Cell;
use std::rc::Rc;
use velox_reactive::{Signal, Subscription, SubscriptionBag};

#[test]
fn signal_get_set() {
    let signal = Signal::new(0);
    assert_eq!(signal.get(), 0);
    signal.set(42);
    assert_eq!(signal.get(), 42);
}

#[test]
fn signal_clone_shares_state() {
    let a = Signal::new(0);
    let b = a.clone();
    a.set(10);
    assert_eq!(b.get(), 10);
}

#[test]
fn signal_subscribe_notifies() {
    let signal = Signal::new(0);
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    let _sub = signal.subscribe(move |val| {
        received_clone.set(*val);
    });
    signal.set(5);
    assert_eq!(received.get(), 5);
}

#[test]
fn subscription_drop_stops_notifications() {
    let signal = Signal::new(0);
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    let sub = signal.subscribe(move |val| {
        received_clone.set(*val);
    });
    signal.set(1);
    assert_eq!(received.get(), 1);
    drop(sub);
    signal.set(2);
    assert_eq!(received.get(), 1);
}

#[test]
fn subscription_bag_drops_all() {
    let signal = Signal::new(0);
    let count = Rc::new(Cell::new(0));
    let mut bag = SubscriptionBag::new();
    for _ in 0..3 {
        let count = count.clone();
        bag.add(signal.subscribe(move |_| {
            count.set(count.get() + 1);
        }));
    }
    signal.set(1);
    assert_eq!(count.get(), 3);
    drop(bag);
    signal.set(2);
    assert_eq!(count.get(), 3);
}

#[test]
fn signal_read_inside_subscriber_no_panic() {
    let signal = Signal::new(0);
    let signal_clone = signal.clone();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    let _sub = signal.subscribe(move |_| {
        received_clone.set(signal_clone.get());
    });
    signal.set(42);
    assert_eq!(received.get(), 42);
}

#[test]
fn signal_update_with_closure() {
    let signal = Signal::new(vec![1, 2, 3]);
    signal.update(|v| v.push(4));
    assert_eq!(signal.get(), vec![1, 2, 3, 4]);
}

#[test]
fn signal_version_increments() {
    let signal = Signal::new(0);
    assert_eq!(signal.version(), 0);
    signal.set(1);
    assert_eq!(signal.version(), 1);
    signal.set(2);
    assert_eq!(signal.version(), 2);
}
