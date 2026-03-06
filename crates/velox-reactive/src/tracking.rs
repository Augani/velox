use std::cell::RefCell;

use crate::subscription::Subscription;

pub(crate) type SubscribeFn = Box<dyn FnOnce(Box<dyn Fn()>) -> Subscription>;

thread_local! {
    static TRACKING_STACK: RefCell<Vec<Vec<SubscribeFn>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn start_tracking() {
    TRACKING_STACK.with(|stack| {
        stack.borrow_mut().push(Vec::new());
    });
}

pub(crate) fn stop_tracking() -> Vec<SubscribeFn> {
    TRACKING_STACK.with(|stack| stack.borrow_mut().pop().unwrap_or_default())
}

pub(crate) fn is_tracking() -> bool {
    TRACKING_STACK.with(|stack| !stack.borrow().is_empty())
}

pub(crate) fn track_signal(subscribe_fn: SubscribeFn) {
    TRACKING_STACK.with(|stack| {
        let mut s = stack.borrow_mut();
        if let Some(current) = s.last_mut() {
            current.push(subscribe_fn);
        }
    });
}
