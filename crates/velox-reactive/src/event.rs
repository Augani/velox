use std::cell::RefCell;
use std::rc::Rc;

use crate::subscription::{Subscription, SubscriptionFlag};

struct SubscriberEntry<T> {
    flag: SubscriptionFlag,
    callback: Rc<dyn Fn(&T)>,
}

struct EventInner<T> {
    subscribers: Vec<SubscriberEntry<T>>,
}

pub struct Event<T> {
    inner: Rc<RefCell<EventInner<T>>>,
}

impl<T> Event<T> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(EventInner {
                subscribers: Vec::new(),
            })),
        }
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

    pub fn emit(&self, value: T) {
        let to_notify: Vec<(SubscriptionFlag, Rc<dyn Fn(&T)>)> = {
            let inner = self.inner.borrow();
            inner
                .subscribers
                .iter()
                .filter(|e| e.flag.is_active())
                .map(|e| (e.flag.clone(), e.callback.clone()))
                .collect()
        };
        for (flag, callback) in &to_notify {
            if flag.is_active() {
                callback(&value);
            }
        }
        self.inner
            .borrow_mut()
            .subscribers
            .retain(|e| e.flag.is_active());
    }
}

impl<T> Default for Event<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}
