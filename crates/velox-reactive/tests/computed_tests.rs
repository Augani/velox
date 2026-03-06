use std::cell::Cell;
use std::rc::Rc;
use velox_reactive::{Computed, Signal};

#[test]
fn computed_derives_value() {
    let signal = Signal::new(2);
    let computed = Computed::new({
        let signal = signal.clone();
        move || signal.get() * 3
    });
    assert_eq!(computed.get(), 6);
}

#[test]
fn computed_updates_when_source_changes() {
    let signal = Signal::new(10);
    let computed = Computed::new({
        let signal = signal.clone();
        move || signal.get() + 5
    });
    assert_eq!(computed.get(), 15);
    signal.set(20);
    assert_eq!(computed.get(), 25);
}

#[test]
fn computed_is_lazy() {
    let call_count = Rc::new(Cell::new(0));
    let signal = Signal::new(1);
    let _computed = Computed::new({
        let signal = signal.clone();
        let call_count = call_count.clone();
        move || {
            call_count.set(call_count.get() + 1);
            signal.get()
        }
    });
    assert_eq!(call_count.get(), 0);
}

#[test]
fn computed_caches_value() {
    let call_count = Rc::new(Cell::new(0));
    let signal = Signal::new(1);
    let computed = Computed::new({
        let signal = signal.clone();
        let call_count = call_count.clone();
        move || {
            call_count.set(call_count.get() + 1);
            signal.get()
        }
    });
    assert_eq!(computed.get(), 1);
    assert_eq!(call_count.get(), 1);
    assert_eq!(computed.get(), 1);
    assert_eq!(call_count.get(), 1);
}

#[test]
fn computed_invalidates_on_signal_change() {
    let call_count = Rc::new(Cell::new(0));
    let signal = Signal::new(1);
    let computed = Computed::new({
        let signal = signal.clone();
        let call_count = call_count.clone();
        move || {
            call_count.set(call_count.get() + 1);
            signal.get()
        }
    });
    assert_eq!(computed.get(), 1);
    assert_eq!(call_count.get(), 1);
    signal.set(2);
    assert_eq!(computed.get(), 2);
    assert_eq!(call_count.get(), 2);
}

#[test]
fn computed_clone_shares_state() {
    let signal = Signal::new(5);
    let computed = Computed::new({
        let signal = signal.clone();
        move || signal.get() * 2
    });
    let computed2 = computed.clone();
    assert_eq!(computed.get(), 10);
    assert_eq!(computed2.get(), 10);
    signal.set(7);
    assert_eq!(computed2.get(), 14);
}

#[test]
fn computed_chains() {
    let signal = Signal::new(2);
    let doubled = Computed::new({
        let signal = signal.clone();
        move || signal.get() * 2
    });
    let quadrupled = Computed::new({
        let doubled = doubled.clone();
        move || doubled.get() * 2
    });
    assert_eq!(quadrupled.get(), 8);
    signal.set(3);
    assert_eq!(quadrupled.get(), 12);
}
