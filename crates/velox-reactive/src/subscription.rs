use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct SubscriptionFlag {
    active: Rc<Cell<bool>>,
}

impl SubscriptionFlag {
    pub(crate) fn new() -> Self {
        Self {
            active: Rc::new(Cell::new(true)),
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    fn deactivate(&self) {
        self.active.set(false);
    }
}

pub struct Subscription {
    flag: SubscriptionFlag,
}

impl Subscription {
    pub(crate) fn new(flag: SubscriptionFlag) -> Self {
        Self { flag }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.flag.deactivate();
    }
}

pub struct SubscriptionBag {
    subscriptions: Vec<Subscription>,
}

impl SubscriptionBag {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
        }
    }

    pub fn add(&mut self, subscription: Subscription) {
        self.subscriptions.push(subscription);
    }
}

impl Default for SubscriptionBag {
    fn default() -> Self {
        Self::new()
    }
}
