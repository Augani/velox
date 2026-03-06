use std::cell::Cell;
use std::rc::Rc;
use velox_reactive::Event;

#[test]
fn event_emit_and_subscribe() {
    let event = Event::<i32>::new();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    let _sub = event.subscribe(move |val| {
        received_clone.set(*val);
    });
    event.emit(42);
    assert_eq!(received.get(), 42);
}

#[test]
fn event_multiple_subscribers() {
    let event = Event::<i32>::new();
    let count = Rc::new(Cell::new(0));
    let _sub1 = {
        let count = count.clone();
        event.subscribe(move |_| count.set(count.get() + 1))
    };
    let _sub2 = {
        let count = count.clone();
        event.subscribe(move |_| count.set(count.get() + 1))
    };
    event.emit(1);
    assert_eq!(count.get(), 2);
}

#[test]
fn event_drop_subscription_stops_notify() {
    let event = Event::<i32>::new();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    let sub = event.subscribe(move |val| {
        received_clone.set(*val);
    });
    event.emit(1);
    assert_eq!(received.get(), 1);
    drop(sub);
    event.emit(2);
    assert_eq!(received.get(), 1);
}

#[test]
fn event_no_value_storage() {
    let event = Event::<i32>::new();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    event.emit(99);
    let _sub = event.subscribe(move |val| {
        received_clone.set(*val);
    });
    assert_eq!(received.get(), 0);
}

#[test]
fn event_unit_type() {
    let event = Event::<()>::new();
    let fired = Rc::new(Cell::new(false));
    let fired_clone = fired.clone();
    let _sub = event.subscribe(move |_| {
        fired_clone.set(true);
    });
    event.emit(());
    assert!(fired.get());
}

#[test]
fn event_clone_shares_state() {
    let event = Event::<i32>::new();
    let event2 = event.clone();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();
    let _sub = event.subscribe(move |val| {
        received_clone.set(*val);
    });
    event2.emit(7);
    assert_eq!(received.get(), 7);
}
