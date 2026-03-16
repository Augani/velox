pub(crate) mod batch;
mod computed;
mod event;
mod signal;
pub(crate) mod subscription;
pub(crate) mod tracking;

pub use batch::{Batch, batch};
pub use computed::Computed;
pub use event::Event;
pub use signal::Signal;
pub use subscription::{Subscription, SubscriptionBag};

pub fn track_render<R>(
    f: impl FnOnce() -> R,
    on_change: impl Fn() + 'static,
) -> (R, Vec<Subscription>) {
    tracking::start_tracking();
    let result = f();
    let subscribe_fns = tracking::stop_tracking();
    let on_change = std::rc::Rc::new(on_change);
    let subs = subscribe_fns
        .into_iter()
        .map(|sf| {
            let cb = on_change.clone();
            sf(Box::new(move || cb()))
        })
        .collect();
    (result, subs)
}
