use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static BATCH_DEPTH: RefCell<u32> = const { RefCell::new(0) };
    static PENDING: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

pub fn batch(f: impl FnOnce()) {
    BATCH_DEPTH.with(|depth| {
        *depth.borrow_mut() += 1;
    });

    f();

    let should_flush = BATCH_DEPTH.with(|depth| {
        let mut d = depth.borrow_mut();
        *d -= 1;
        *d == 0
    });

    if should_flush {
        flush();
    }
}

pub(crate) fn is_batching() -> bool {
    BATCH_DEPTH.with(|depth| *depth.borrow() > 0)
}

pub(crate) fn enqueue_notify(notify_fn: Rc<dyn Fn()>) {
    PENDING.with(|pending| {
        let mut p = pending.borrow_mut();
        let ptr = Rc::as_ptr(&notify_fn);
        if !p
            .iter()
            .any(|existing| std::ptr::addr_eq(Rc::as_ptr(existing), ptr))
        {
            p.push(notify_fn);
        }
    });
}

fn flush() {
    let pending = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    for notify_fn in pending {
        notify_fn();
    }
}

pub struct Batch;

impl Batch {
    pub fn run(f: impl FnOnce()) {
        batch(f);
    }
}
