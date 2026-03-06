use std::cell::RefCell;
use std::rc::Rc;

use crate::batch;
use crate::subscription::{Subscription, SubscriptionFlag};
use crate::tracking;

type Callback<T> = Rc<dyn Fn(&T)>;
type NotifyEntry<T> = (SubscriptionFlag, Callback<T>);

struct SubscriberEntry<T> {
    flag: SubscriptionFlag,
    callback: Callback<T>,
}

struct SignalInner<T> {
    value: T,
    version: u64,
    subscribers: Vec<SubscriberEntry<T>>,
}

pub struct Signal<T> {
    inner: Rc<RefCell<SignalInner<T>>>,
    notify_fn: Rc<dyn Fn()>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(value: T) -> Self {
        let inner = Rc::new(RefCell::new(SignalInner {
            value,
            version: 0,
            subscribers: Vec::new(),
        }));
        let inner_ref = Rc::clone(&inner);
        let notify_fn: Rc<dyn Fn()> = Rc::new(move || {
            Self::do_notify(&inner_ref);
        });
        Self { inner, notify_fn }
    }

    pub fn get(&self) -> T {
        if tracking::is_tracking() {
            let signal = self.clone();
            tracking::track_signal(Box::new(move |invalidate: Box<dyn Fn()>| {
                signal.subscribe(move |_| invalidate())
            }));
        }
        self.inner.borrow().value.clone()
    }

    pub fn set(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version += 1;
        }
        self.schedule_notify();
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let mut inner = self.inner.borrow_mut();
            f(&mut inner.value);
            inner.version += 1;
        }
        self.schedule_notify();
    }

    pub fn version(&self) -> u64 {
        self.inner.borrow().version
    }

    pub fn subscribe(&self, callback: impl Fn(&T) + 'static) -> Subscription {
        let flag = SubscriptionFlag::new();
        let entry = SubscriberEntry {
            flag: flag.clone(),
            callback: Rc::new(callback),
        };
        self.inner.borrow_mut().subscribers.push(entry);
        Subscription::new(flag)
    }

    fn schedule_notify(&self) {
        if batch::is_batching() {
            batch::enqueue_notify(Rc::clone(&self.notify_fn));
        } else {
            Self::do_notify(&self.inner);
        }
    }

    fn do_notify(inner: &Rc<RefCell<SignalInner<T>>>) {
        let to_notify: Vec<NotifyEntry<T>> = {
            let borrowed = inner.borrow();
            borrowed
                .subscribers
                .iter()
                .filter(|e| e.flag.is_active())
                .map(|e| (e.flag.clone(), e.callback.clone()))
                .collect()
        };
        let value = inner.borrow().value.clone();
        for (flag, callback) in &to_notify {
            if flag.is_active() {
                callback(&value);
            }
        }
        inner
            .borrow_mut()
            .subscribers
            .retain(|e| e.flag.is_active());
    }
}

impl<T: Clone + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            notify_fn: Rc::clone(&self.notify_fn),
        }
    }
}
