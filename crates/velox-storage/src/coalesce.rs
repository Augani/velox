use std::sync::{Arc, Mutex};
use std::time::Duration;

type CoalescedAction = Box<dyn FnOnce() + Send>;

pub struct WriteCoalescer {
    state: Arc<Mutex<CoalescerState>>,
    delay: Duration,
}

struct CoalescerState {
    pending: bool,
    latest_action: Option<CoalescedAction>,
}

impl WriteCoalescer {
    pub fn new(delay: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(CoalescerState {
                pending: false,
                latest_action: None,
            })),
            delay,
        }
    }

    pub fn schedule(
        &self,
        handle: &tokio::runtime::Handle,
        action: impl FnOnce() + Send + 'static,
    ) {
        let mut locked = self.state.lock().unwrap_or_else(|e| e.into_inner());
        locked.latest_action = Some(Box::new(action));
        if locked.pending {
            return;
        }
        locked.pending = true;
        drop(locked);

        let state = Arc::clone(&self.state);
        let delay = self.delay;
        handle.spawn(async move {
            tokio::time::sleep(delay).await;
            let action = {
                let mut locked = state.lock().unwrap_or_else(|e| e.into_inner());
                locked.pending = false;
                locked.latest_action.take()
            };
            if let Some(action) = action {
                action();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalescer_keeps_latest_action() {
        let coalescer = WriteCoalescer::new(Duration::from_secs(60));
        let rt = tokio::runtime::Runtime::new().unwrap();

        let value = Arc::new(Mutex::new(0u32));

        let v1 = Arc::clone(&value);
        coalescer.schedule(rt.handle(), move || {
            *v1.lock().unwrap() = 1;
        });

        let v2 = Arc::clone(&value);
        coalescer.schedule(rt.handle(), move || {
            *v2.lock().unwrap() = 2;
        });

        let state = coalescer.state.lock().unwrap();
        assert!(state.pending);
        assert!(state.latest_action.is_some());
    }

    #[test]
    fn coalescer_executes_latest() {
        let coalescer = WriteCoalescer::new(Duration::from_millis(10));
        let rt = tokio::runtime::Runtime::new().unwrap();
        let value = Arc::new(Mutex::new(0u32));

        let v1 = Arc::clone(&value);
        coalescer.schedule(rt.handle(), move || {
            *v1.lock().unwrap() = 1;
        });

        let v2 = Arc::clone(&value);
        coalescer.schedule(rt.handle(), move || {
            *v2.lock().unwrap() = 2;
        });

        std::thread::sleep(Duration::from_millis(50));
        rt.block_on(async { tokio::task::yield_now().await });
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(*value.lock().unwrap(), 2);
    }
}
