use std::cell::RefCell;
use std::rc::Rc;

use crate::subscription::{Subscription, SubscriptionFlag};
use crate::tracking;

struct InvalidationEntry {
    flag: SubscriptionFlag,
    callback: Box<dyn Fn()>,
}

struct ComputedInner<T> {
    compute: Box<dyn Fn() -> T>,
    cached: Option<T>,
    dirty: bool,
    source_subscriptions: Vec<Subscription>,
    invalidation_listeners: Vec<InvalidationEntry>,
}

impl<T> ComputedInner<T> {
    fn mark_dirty(&mut self) {
        if !self.dirty {
            self.dirty = true;
            for entry in &self.invalidation_listeners {
                if entry.flag.is_active() {
                    (entry.callback)();
                }
            }
            self.invalidation_listeners.retain(|e| e.flag.is_active());
        }
    }
}

pub struct Computed<T> {
    inner: Rc<RefCell<ComputedInner<T>>>,
}

impl<T: Clone + 'static> Computed<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ComputedInner {
                compute: Box::new(compute),
                cached: None,
                dirty: true,
                source_subscriptions: Vec::new(),
                invalidation_listeners: Vec::new(),
            })),
        }
    }

    pub fn get(&self) -> T {
        if tracking::is_tracking() {
            let computed = self.clone();
            tracking::track_signal(Box::new(move |invalidate: Box<dyn Fn()>| {
                computed.on_invalidate(invalidate)
            }));
        }

        let needs_eval = {
            let inner = self.inner.borrow();
            inner.dirty || inner.cached.is_none()
        };

        if needs_eval {
            self.evaluate();
        }

        self.inner.borrow().cached.clone().unwrap()
    }

    fn on_invalidate(&self, callback: Box<dyn Fn()>) -> Subscription {
        let flag = SubscriptionFlag::new();
        let entry = InvalidationEntry {
            flag: flag.clone(),
            callback,
        };
        self.inner.borrow_mut().invalidation_listeners.push(entry);
        Subscription::new(flag)
    }

    fn evaluate(&self) {
        tracking::start_tracking();

        let value = {
            let inner = self.inner.borrow();
            (inner.compute)()
        };

        let subscribe_fns = tracking::stop_tracking();

        let weak = Rc::downgrade(&self.inner);
        let mut subscriptions = Vec::new();
        for subscribe_fn in subscribe_fns {
            let weak = weak.clone();
            let sub = subscribe_fn(Box::new(move || {
                if let Some(inner) = weak.upgrade() {
                    inner.borrow_mut().mark_dirty();
                }
            }));
            subscriptions.push(sub);
        }

        let mut inner = self.inner.borrow_mut();
        inner.cached = Some(value);
        inner.dirty = false;
        inner.source_subscriptions = subscriptions;
    }
}

impl<T: Clone + 'static> Clone for Computed<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}
