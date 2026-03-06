use std::cell::RefCell;
use std::rc::Rc;

use crate::subscription::{Subscription, SubscriptionFlag};

struct SubscriberEntry<T> {
    flag: SubscriptionFlag,
    callback: Rc<dyn Fn(&T)>,
}

struct SignalInner<T> {
    value: T,
    version: u64,
    subscribers: Vec<SubscriberEntry<T>>,
}

pub struct Signal<T> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

impl<T: Clone> Signal<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SignalInner {
                value,
                version: 0,
                subscribers: Vec::new(),
            })),
        }
    }

    pub fn get(&self) -> T {
        self.inner.borrow().value.clone()
    }

    pub fn set(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version += 1;
        }
        self.notify();
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let mut inner = self.inner.borrow_mut();
            f(&mut inner.value);
            inner.version += 1;
        }
        self.notify();
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

    fn notify(&self) {
        let to_notify: Vec<(SubscriptionFlag, Rc<dyn Fn(&T)>)> = {
            let inner = self.inner.borrow();
            inner
                .subscribers
                .iter()
                .filter(|e| e.flag.is_active())
                .map(|e| (e.flag.clone(), e.callback.clone()))
                .collect()
        };
        let value = self.get();
        for (flag, callback) in &to_notify {
            if flag.is_active() {
                callback(&value);
            }
        }
        self.cleanup_dead_subscribers();
    }

    fn cleanup_dead_subscribers(&self) {
        self.inner
            .borrow_mut()
            .subscribers
            .retain(|e| e.flag.is_active());
    }
}

impl<T: Clone> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}
