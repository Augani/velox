# Phase 1: Runtime and Window Shell — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the foundational Rust workspace with 6 crates: reactive state primitives, platform abstraction, runtime with task scheduling, window management, application assembly, and a facade crate.

**Architecture:** Workspace of focused crates. `velox-reactive` is pure Rust with no framework deps. `velox-platform` defines traits with macOS stubs. `velox-runtime` owns task scheduling (UI queue, tokio-backed IO, thread pool compute, delayed execution) with cancellation and deliver-to-main-thread pattern. `velox-window` wraps winit windows. `velox-app` implements winit's `ApplicationHandler` to tie it all together. `velox` is the facade.

**Tech Stack:** Rust 1.92+, winit 0.30, tokio 1, std threading for compute pool

---

### Task 1: Workspace Scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/velox/Cargo.toml`
- Create: `crates/velox/src/lib.rs`
- Create: `crates/velox-reactive/Cargo.toml`
- Create: `crates/velox-reactive/src/lib.rs`
- Create: `crates/velox-platform/Cargo.toml`
- Create: `crates/velox-platform/src/lib.rs`
- Create: `crates/velox-runtime/Cargo.toml`
- Create: `crates/velox-runtime/src/lib.rs`
- Create: `crates/velox-window/Cargo.toml`
- Create: `crates/velox-window/src/lib.rs`
- Create: `crates/velox-app/Cargo.toml`
- Create: `crates/velox-app/src/lib.rs`

**Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/velox",
    "crates/velox-reactive",
    "crates/velox-platform",
    "crates/velox-runtime",
    "crates/velox-window",
    "crates/velox-app",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
rust-version = "1.85"

[workspace.dependencies]
velox = { path = "crates/velox" }
velox-reactive = { path = "crates/velox-reactive" }
velox-platform = { path = "crates/velox-platform" }
velox-runtime = { path = "crates/velox-runtime" }
velox-window = { path = "crates/velox-window" }
velox-app = { path = "crates/velox-app" }
winit = "0.30"
tokio = { version = "1", features = ["rt-multi-thread", "time", "sync"] }
```

**Step 2: Create each crate's Cargo.toml and minimal lib.rs**

`crates/velox-reactive/Cargo.toml`:
```toml
[package]
name = "velox-reactive"
version.workspace = true
edition.workspace = true

[dependencies]
```

`crates/velox-reactive/src/lib.rs`:
```rust
pub fn placeholder() {}
```

`crates/velox-platform/Cargo.toml`:
```toml
[package]
name = "velox-platform"
version.workspace = true
edition.workspace = true

[dependencies]
```

`crates/velox-platform/src/lib.rs`:
```rust
pub fn placeholder() {}
```

`crates/velox-runtime/Cargo.toml`:
```toml
[package]
name = "velox-runtime"
version.workspace = true
edition.workspace = true

[dependencies]
velox-platform = { workspace = true }
tokio = { workspace = true }
```

`crates/velox-runtime/src/lib.rs`:
```rust
pub fn placeholder() {}
```

`crates/velox-window/Cargo.toml`:
```toml
[package]
name = "velox-window"
version.workspace = true
edition.workspace = true

[dependencies]
velox-platform = { workspace = true }
winit = { workspace = true }
```

`crates/velox-window/src/lib.rs`:
```rust
pub fn placeholder() {}
```

`crates/velox-app/Cargo.toml`:
```toml
[package]
name = "velox-app"
version.workspace = true
edition.workspace = true

[dependencies]
velox-reactive = { workspace = true }
velox-platform = { workspace = true }
velox-runtime = { workspace = true }
velox-window = { workspace = true }
winit = { workspace = true }
```

`crates/velox-app/src/lib.rs`:
```rust
pub fn placeholder() {}
```

`crates/velox/Cargo.toml`:
```toml
[package]
name = "velox"
version.workspace = true
edition.workspace = true

[dependencies]
velox-reactive = { workspace = true }
velox-platform = { workspace = true }
velox-runtime = { workspace = true }
velox-window = { workspace = true }
velox-app = { workspace = true }
```

`crates/velox/src/lib.rs`:
```rust
pub use velox_reactive as reactive;
pub use velox_platform as platform;
pub use velox_runtime as runtime;
pub use velox_window as window;
pub use velox_app as app;
```

**Step 3: Build and verify**

Run: `cargo build`
Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git init
echo "target/" > .gitignore
git add -A
git commit -m "feat: scaffold velox workspace with 6 crates"
```

---

### Task 2: velox-reactive — Signal + Subscription

**Files:**
- Create: `crates/velox-reactive/src/signal.rs`
- Create: `crates/velox-reactive/src/subscription.rs`
- Modify: `crates/velox-reactive/src/lib.rs`
- Create: `crates/velox-reactive/tests/signal_tests.rs`

**Step 1: Write failing tests**

`crates/velox-reactive/tests/signal_tests.rs`:
```rust
use velox_reactive::{Signal, Subscription, SubscriptionBag};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn signal_get_set() {
    let signal = Signal::new(0);
    assert_eq!(signal.get(), 0);
    signal.set(42);
    assert_eq!(signal.get(), 42);
}

#[test]
fn signal_clone_shares_state() {
    let a = Signal::new(0);
    let b = a.clone();
    a.set(10);
    assert_eq!(b.get(), 10);
}

#[test]
fn signal_subscribe_notifies() {
    let signal = Signal::new(0);
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    let _sub = signal.subscribe(move |val| {
        received_clone.set(*val);
    });

    signal.set(5);
    assert_eq!(received.get(), 5);
}

#[test]
fn signal_subscribe_gets_current_value() {
    let signal = Signal::new(0);
    signal.set(99);

    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    signal.set(100);
    assert_eq!(received.get(), 0);

    let _sub = signal.subscribe(move |val| {
        received_clone.set(*val);
    });
    signal.set(200);
    assert_eq!(received.get(), 200);
}

#[test]
fn subscription_drop_stops_notifications() {
    let signal = Signal::new(0);
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    let sub = signal.subscribe(move |val| {
        received_clone.set(*val);
    });

    signal.set(1);
    assert_eq!(received.get(), 1);

    drop(sub);

    signal.set(2);
    assert_eq!(received.get(), 1);
}

#[test]
fn subscription_bag_drops_all() {
    let signal = Signal::new(0);
    let count = Rc::new(Cell::new(0));

    let mut bag = SubscriptionBag::new();

    for _ in 0..3 {
        let count = count.clone();
        bag.add(signal.subscribe(move |_| {
            count.set(count.get() + 1);
        }));
    }

    signal.set(1);
    assert_eq!(count.get(), 3);

    drop(bag);

    signal.set(2);
    assert_eq!(count.get(), 3);
}

#[test]
fn signal_read_inside_subscriber_no_panic() {
    let signal = Signal::new(0);
    let signal_clone = signal.clone();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    let _sub = signal.subscribe(move |_| {
        received_clone.set(signal_clone.get());
    });

    signal.set(42);
    assert_eq!(received.get(), 42);
}

#[test]
fn signal_update_with_closure() {
    let signal = Signal::new(vec![1, 2, 3]);
    signal.update(|v| v.push(4));
    assert_eq!(signal.get(), vec![1, 2, 3, 4]);
}

#[test]
fn signal_version_increments() {
    let signal = Signal::new(0);
    assert_eq!(signal.version(), 0);
    signal.set(1);
    assert_eq!(signal.version(), 1);
    signal.set(2);
    assert_eq!(signal.version(), 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-reactive`
Expected: Compilation error — `Signal`, `Subscription`, `SubscriptionBag` not found.

**Step 3: Implement Subscription**

`crates/velox-reactive/src/subscription.rs`:
```rust
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

    pub(crate) fn deactivate(&self) {
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

    pub fn add(&mut self, sub: Subscription) {
        self.subscriptions.push(sub);
    }

    pub fn clear(&mut self) {
        self.subscriptions.clear();
    }
}

impl Default for SubscriptionBag {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Implement Signal**

`crates/velox-reactive/src/signal.rs`:
```rust
use std::cell::RefCell;
use std::rc::Rc;

use crate::subscription::{Subscription, SubscriptionFlag};

struct SubscriberEntry<T> {
    flag: SubscriptionFlag,
    callback: Rc<dyn Fn(&T)>,
}

struct SignalInner<T: Clone + 'static> {
    value: T,
    version: u64,
    subscribers: Vec<SubscriberEntry<T>>,
}

#[derive(Clone)]
pub struct Signal<T: Clone + 'static> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

impl<T: Clone + 'static> Signal<T> {
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
        let mut inner = self.inner.borrow_mut();
        inner.subscribers.push(SubscriberEntry {
            flag: flag.clone(),
            callback: Rc::new(callback),
        });
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
        let mut inner = self.inner.borrow_mut();
        inner.subscribers.retain(|e| e.flag.is_active());
    }
}
```

**Step 5: Update lib.rs**

`crates/velox-reactive/src/lib.rs`:
```rust
mod signal;
mod subscription;

pub use signal::Signal;
pub use subscription::{Subscription, SubscriptionBag};
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p velox-reactive`
Expected: All 9 tests pass.

**Step 7: Commit**

```bash
git add crates/velox-reactive/
git commit -m "feat(reactive): implement Signal with subscription lifecycle"
```

---

### Task 3: velox-reactive — Computed, Event, Batch

**Files:**
- Create: `crates/velox-reactive/src/computed.rs`
- Create: `crates/velox-reactive/src/event.rs`
- Create: `crates/velox-reactive/src/batch.rs`
- Modify: `crates/velox-reactive/src/lib.rs`
- Create: `crates/velox-reactive/tests/computed_tests.rs`
- Create: `crates/velox-reactive/tests/event_tests.rs`
- Create: `crates/velox-reactive/tests/batch_tests.rs`

**Step 1: Write failing tests for Computed**

`crates/velox-reactive/tests/computed_tests.rs`:
```rust
use velox_reactive::{Computed, Signal};

#[test]
fn computed_derives_from_signal() {
    let count = Signal::new(5);
    let doubled = Computed::new({
        let count = count.clone();
        move || count.get() * 2
    });

    assert_eq!(doubled.get(), 10);
    count.set(10);
    assert_eq!(doubled.get(), 20);
}

#[test]
fn computed_derives_from_multiple_signals() {
    let a = Signal::new(2);
    let b = Signal::new(3);
    let sum = Computed::new({
        let a = a.clone();
        let b = b.clone();
        move || a.get() + b.get()
    });

    assert_eq!(sum.get(), 5);
    a.set(10);
    assert_eq!(sum.get(), 13);
    b.set(20);
    assert_eq!(sum.get(), 30);
}

#[test]
fn computed_clone_shares_computation() {
    let count = Signal::new(1);
    let doubled = Computed::new({
        let count = count.clone();
        move || count.get() * 2
    });
    let doubled2 = doubled.clone();

    assert_eq!(doubled.get(), 2);
    assert_eq!(doubled2.get(), 2);
    count.set(5);
    assert_eq!(doubled.get(), 10);
    assert_eq!(doubled2.get(), 10);
}
```

**Step 2: Write failing tests for Event**

`crates/velox-reactive/tests/event_tests.rs`:
```rust
use velox_reactive::Event;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn event_emits_to_subscribers() {
    let event: Event<i32> = Event::new();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    let _sub = event.subscribe(move |val| {
        received_clone.set(*val);
    });

    event.emit(42);
    assert_eq!(received.get(), 42);
}

#[test]
fn event_multiple_subscribers() {
    let event: Event<i32> = Event::new();
    let count = Rc::new(Cell::new(0));

    let c1 = count.clone();
    let _s1 = event.subscribe(move |_| c1.set(c1.get() + 1));
    let c2 = count.clone();
    let _s2 = event.subscribe(move |_| c2.set(c2.get() + 1));

    event.emit(1);
    assert_eq!(count.get(), 2);
}

#[test]
fn event_drop_subscription_stops_delivery() {
    let event: Event<i32> = Event::new();
    let received = Rc::new(Cell::new(0));
    let received_clone = received.clone();

    let sub = event.subscribe(move |val| {
        received_clone.set(*val);
    });

    event.emit(1);
    assert_eq!(received.get(), 1);

    drop(sub);
    event.emit(2);
    assert_eq!(received.get(), 1);
}
```

**Step 3: Write failing tests for Batch**

`crates/velox-reactive/tests/batch_tests.rs`:
```rust
use velox_reactive::{Signal, Batch};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn batch_defers_notifications() {
    let signal = Signal::new(0);
    let notify_count = Rc::new(Cell::new(0));
    let nc = notify_count.clone();

    let _sub = signal.subscribe(move |_| {
        nc.set(nc.get() + 1);
    });

    Batch::run(|| {
        signal.set(1);
        signal.set(2);
        signal.set(3);
        assert_eq!(notify_count.get(), 0);
    });

    assert_eq!(notify_count.get(), 1);
    assert_eq!(signal.get(), 3);
}

#[test]
fn batch_multiple_signals() {
    let a = Signal::new(0);
    let b = Signal::new(0);
    let total_notifications = Rc::new(Cell::new(0));

    let nc = total_notifications.clone();
    let _s1 = a.subscribe(move |_| nc.set(nc.get() + 1));
    let nc = total_notifications.clone();
    let _s2 = b.subscribe(move |_| nc.set(nc.get() + 1));

    Batch::run(|| {
        a.set(10);
        b.set(20);
    });

    assert_eq!(total_notifications.get(), 2);
    assert_eq!(a.get(), 10);
    assert_eq!(b.get(), 20);
}
```

**Step 4: Run tests to verify they fail**

Run: `cargo test -p velox-reactive`
Expected: Compilation errors — `Computed`, `Event`, `Batch` not found.

**Step 5: Implement Computed**

`crates/velox-reactive/src/computed.rs`:
```rust
use std::rc::Rc;

#[derive(Clone)]
pub struct Computed<T: Clone + 'static> {
    compute: Rc<dyn Fn() -> T>,
}

impl<T: Clone + 'static> Computed<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        Self {
            compute: Rc::new(compute),
        }
    }

    pub fn get(&self) -> T {
        (self.compute)()
    }
}
```

**Step 6: Implement Event**

`crates/velox-reactive/src/event.rs`:
```rust
use std::cell::RefCell;
use std::rc::Rc;

use crate::subscription::{Subscription, SubscriptionFlag};

struct EventSubscriber<T> {
    flag: SubscriptionFlag,
    callback: Rc<dyn Fn(&T)>,
}

struct EventInner<T: 'static> {
    subscribers: Vec<EventSubscriber<T>>,
}

#[derive(Clone)]
pub struct Event<T: 'static> {
    inner: Rc<RefCell<EventInner<T>>>,
}

impl<T: 'static> Event<T> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(EventInner {
                subscribers: Vec::new(),
            })),
        }
    }

    pub fn subscribe(&self, callback: impl Fn(&T) + 'static) -> Subscription {
        let flag = SubscriptionFlag::new();
        let mut inner = self.inner.borrow_mut();
        inner.subscribers.push(EventSubscriber {
            flag: flag.clone(),
            callback: Rc::new(callback),
        });
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

        let mut inner = self.inner.borrow_mut();
        inner.subscribers.retain(|e| e.flag.is_active());
    }
}

impl<T: 'static> Default for Event<T> {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 7: Implement Batch**

Batch requires thread-local state to track whether we're inside a batch and which signals were dirtied.

`crates/velox-reactive/src/batch.rs`:
```rust
use std::cell::RefCell;

thread_local! {
    static BATCH_DEPTH: RefCell<u32> = const { RefCell::new(0) };
    static PENDING_NOTIFICATIONS: RefCell<Vec<Box<dyn FnOnce()>>> = const { RefCell::new(Vec::new()) };
}

pub struct Batch;

impl Batch {
    pub fn run(f: impl FnOnce()) {
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
            let pending = PENDING_NOTIFICATIONS.with(|p| std::mem::take(&mut *p.borrow_mut()));
            for notification in pending {
                notification();
            }
        }
    }

    pub(crate) fn is_batching() -> bool {
        BATCH_DEPTH.with(|depth| *depth.borrow() > 0)
    }

    pub(crate) fn defer_notification(f: impl FnOnce() + 'static) {
        PENDING_NOTIFICATIONS.with(|p| {
            p.borrow_mut().push(Box::new(f));
        });
    }
}
```

**Step 8: Update Signal to support batching**

Modify `crates/velox-reactive/src/signal.rs` — replace the `set` and `update` methods to check for batch mode:

Replace the `set` method body:
```rust
    pub fn set(&self, value: T) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.value = value;
            inner.version += 1;
        }
        if Batch::is_batching() {
            let this = self.clone();
            Batch::defer_notification(move || this.notify());
        } else {
            self.notify();
        }
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        {
            let mut inner = self.inner.borrow_mut();
            f(&mut inner.value);
            inner.version += 1;
        }
        if Batch::is_batching() {
            let this = self.clone();
            Batch::defer_notification(move || this.notify());
        } else {
            self.notify();
        }
    }
```

Add `use crate::batch::Batch;` to signal.rs imports.

**Step 9: Update lib.rs**

`crates/velox-reactive/src/lib.rs`:
```rust
mod batch;
mod computed;
mod event;
mod signal;
mod subscription;

pub use batch::Batch;
pub use computed::Computed;
pub use event::Event;
pub use signal::Signal;
pub use subscription::{Subscription, SubscriptionBag};
```

**Step 10: Run all tests**

Run: `cargo test -p velox-reactive`
Expected: All tests pass (signal, computed, event, batch).

**Step 11: Commit**

```bash
git add crates/velox-reactive/
git commit -m "feat(reactive): add Computed, Event, and Batch primitives"
```

---

### Task 4: velox-platform — Traits and Stub Implementations

**Files:**
- Create: `crates/velox-platform/src/power.rs`
- Create: `crates/velox-platform/src/app.rs`
- Create: `crates/velox-platform/src/clipboard.rs`
- Create: `crates/velox-platform/src/stub.rs`
- Modify: `crates/velox-platform/src/lib.rs`
- Create: `crates/velox-platform/tests/platform_tests.rs`

**Step 1: Write failing tests**

`crates/velox-platform/tests/platform_tests.rs`:
```rust
use velox_platform::{BatteryState, PowerSource, StubPlatform, PlatformPower};

#[test]
fn stub_power_returns_defaults() {
    let platform = StubPlatform::new();
    assert!(matches!(platform.battery_state(), BatteryState::Unknown));
    assert!(matches!(platform.power_source(), PowerSource::Unknown));
    assert!(!platform.is_low_power_mode());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-platform`
Expected: Compilation error.

**Step 3: Implement platform traits**

`crates/velox-platform/src/power.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BatteryState {
    Unknown,
    Unplugged(f32),
    Charging(f32),
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerSource {
    Battery,
    AC,
    Unknown,
}

pub trait PlatformPower {
    fn battery_state(&self) -> BatteryState;
    fn power_source(&self) -> PowerSource;
    fn is_low_power_mode(&self) -> bool;
}
```

`crates/velox-platform/src/app.rs`:
```rust
pub trait PlatformApp {
    fn hide(&self);
    fn show(&self);
    fn set_badge(&self, text: Option<&str>);
}
```

`crates/velox-platform/src/clipboard.rs`:
```rust
pub trait PlatformClipboard {
    fn read_text(&self) -> Option<String>;
    fn write_text(&self, text: &str);
}
```

`crates/velox-platform/src/stub.rs`:
```rust
use crate::{
    app::PlatformApp,
    clipboard::PlatformClipboard,
    power::{BatteryState, PlatformPower, PowerSource},
};

pub struct StubPlatform;

impl StubPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformPower for StubPlatform {
    fn battery_state(&self) -> BatteryState {
        BatteryState::Unknown
    }

    fn power_source(&self) -> PowerSource {
        PowerSource::Unknown
    }

    fn is_low_power_mode(&self) -> bool {
        false
    }
}

impl PlatformApp for StubPlatform {
    fn hide(&self) {}
    fn show(&self) {}
    fn set_badge(&self, _text: Option<&str>) {}
}

impl PlatformClipboard for StubPlatform {
    fn read_text(&self) -> Option<String> {
        None
    }

    fn write_text(&self, _text: &str) {}
}
```

**Step 4: Update lib.rs**

`crates/velox-platform/src/lib.rs`:
```rust
pub mod app;
pub mod clipboard;
pub mod power;
mod stub;

pub use app::PlatformApp;
pub use clipboard::PlatformClipboard;
pub use power::{BatteryState, PlatformPower, PowerSource};
pub use stub::StubPlatform;
```

**Step 5: Run tests**

Run: `cargo test -p velox-platform`
Expected: All tests pass.

**Step 6: Commit**

```bash
git add crates/velox-platform/
git commit -m "feat(platform): define platform traits with stub implementations"
```

---

### Task 5: velox-runtime — FrameClock + CancellationToken

**Files:**
- Create: `crates/velox-runtime/src/frame_clock.rs`
- Create: `crates/velox-runtime/src/cancellation.rs`
- Modify: `crates/velox-runtime/src/lib.rs`
- Create: `crates/velox-runtime/tests/frame_clock_tests.rs`
- Create: `crates/velox-runtime/tests/cancellation_tests.rs`

**Step 1: Write failing tests**

`crates/velox-runtime/tests/frame_clock_tests.rs`:
```rust
use velox_runtime::FrameClock;
use std::time::Duration;

#[test]
fn frame_clock_starts_at_zero() {
    let clock = FrameClock::new();
    assert_eq!(clock.frame_count(), 0);
}

#[test]
fn frame_clock_increments_on_tick() {
    let mut clock = FrameClock::new();
    clock.tick();
    assert_eq!(clock.frame_count(), 1);
    clock.tick();
    assert_eq!(clock.frame_count(), 2);
}

#[test]
fn frame_clock_tracks_delta() {
    let mut clock = FrameClock::new();
    std::thread::sleep(Duration::from_millis(16));
    clock.tick();
    let delta = clock.delta_time();
    assert!(delta >= Duration::from_millis(10));
    assert!(delta < Duration::from_millis(100));
}
```

`crates/velox-runtime/tests/cancellation_tests.rs`:
```rust
use velox_runtime::CancellationToken;

#[test]
fn token_starts_active() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
}

#[test]
fn token_cancel_sets_flag() {
    let token = CancellationToken::new();
    let token2 = token.clone();
    token.cancel();
    assert!(token.is_cancelled());
    assert!(token2.is_cancelled());
}

#[test]
fn token_drop_does_not_cancel() {
    let token = CancellationToken::new();
    let token2 = token.clone();
    drop(token);
    assert!(!token2.is_cancelled());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-runtime`
Expected: Compilation error.

**Step 3: Implement FrameClock**

`crates/velox-runtime/src/frame_clock.rs`:
```rust
use std::time::{Duration, Instant};

pub struct FrameClock {
    frame_count: u64,
    last_tick: Instant,
    delta: Duration,
    start_time: Instant,
}

impl FrameClock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_count: 0,
            last_tick: now,
            delta: Duration::ZERO,
            start_time: now,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = now - self.last_tick;
        self.last_tick = now;
        self.frame_count += 1;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn delta_time(&self) -> Duration {
        self.delta
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Implement CancellationToken**

`crates/velox-runtime/src/cancellation.rs`:
```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 5: Update lib.rs**

`crates/velox-runtime/src/lib.rs`:
```rust
mod cancellation;
mod frame_clock;

pub use cancellation::CancellationToken;
pub use frame_clock::FrameClock;
```

**Step 6: Run tests**

Run: `cargo test -p velox-runtime`
Expected: All tests pass.

**Step 7: Commit**

```bash
git add crates/velox-runtime/
git commit -m "feat(runtime): add FrameClock and CancellationToken"
```

---

### Task 6: velox-runtime — Task Executors + Deliver Pattern

**Files:**
- Create: `crates/velox-runtime/src/executor/mod.rs`
- Create: `crates/velox-runtime/src/executor/ui_queue.rs`
- Create: `crates/velox-runtime/src/executor/compute_pool.rs`
- Create: `crates/velox-runtime/src/executor/io_executor.rs`
- Create: `crates/velox-runtime/src/executor/deliver.rs`
- Modify: `crates/velox-runtime/src/lib.rs`
- Create: `crates/velox-runtime/tests/executor_tests.rs`

**Step 1: Write failing tests**

`crates/velox-runtime/tests/executor_tests.rs`:
```rust
use velox_runtime::executor::{ComputePool, DeliverQueue, IoExecutor, UiQueue};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn ui_queue_runs_tasks_on_flush() {
    let mut queue = UiQueue::new();
    let (tx, rx) = mpsc::channel();

    queue.push(Box::new(move || {
        tx.send(42).unwrap();
    }));

    assert!(rx.try_recv().is_err());
    queue.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 42);
}

#[test]
fn ui_queue_tasks_spawned_during_flush_run_next_flush() {
    let mut queue = UiQueue::new();
    let (tx, rx) = mpsc::channel();

    let tx_clone = tx.clone();
    let queue_ptr: *mut UiQueue = &mut queue;
    queue.push(Box::new(move || {
        tx_clone.send(1).unwrap();
    }));

    queue.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 1);

    queue.push(Box::new(move || {
        tx.send(2).unwrap();
    }));

    queue.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 2);
}

#[test]
fn compute_pool_runs_work_on_background_thread() {
    let pool = ComputePool::new(2);
    let (tx, rx) = mpsc::channel();

    pool.spawn(move || {
        tx.send(std::thread::current().id()).unwrap();
    });

    let worker_thread = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_ne!(worker_thread, std::thread::current().id());
}

#[test]
fn compute_pool_handles_multiple_tasks() {
    let pool = ComputePool::new(2);
    let (tx, rx) = mpsc::channel();

    for i in 0..10 {
        let tx = tx.clone();
        pool.spawn(move || {
            tx.send(i).unwrap();
        });
    }

    let mut results: Vec<i32> = (0..10)
        .map(|_| rx.recv_timeout(Duration::from_secs(2)).unwrap())
        .collect();
    results.sort();
    assert_eq!(results, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn io_executor_runs_async_work() {
    let executor = IoExecutor::new();
    let (tx, rx) = mpsc::channel();

    executor.spawn(async move {
        tx.send(42).unwrap();
    });

    let result = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(result, 42);
}

#[test]
fn deliver_queue_matches_results_to_callbacks() {
    let mut deliver = DeliverQueue::new();
    let (tx, rx) = mpsc::channel();

    let task_id = deliver.register(move |boxed: Box<dyn std::any::Any + Send>| {
        let value = *boxed.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });

    deliver.send_result(task_id, Box::new(99i32));
    deliver.flush();

    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 99);
}

#[test]
fn deliver_queue_ignores_unregistered_results() {
    let mut deliver = DeliverQueue::new();
    deliver.send_result(999, Box::new(42i32));
    deliver.flush();
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-runtime`
Expected: Compilation error.

**Step 3: Implement UiQueue**

`crates/velox-runtime/src/executor/ui_queue.rs`:
```rust
pub struct UiQueue {
    tasks: Vec<Box<dyn FnOnce()>>,
}

impl UiQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn push(&mut self, task: Box<dyn FnOnce()>) {
        self.tasks.push(task);
    }

    pub fn flush(&mut self) {
        let tasks = std::mem::take(&mut self.tasks);
        for task in tasks {
            task();
        }
    }
}

impl Default for UiQueue {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Implement ComputePool**

`crates/velox-runtime/src/executor/compute_pool.rs`:
```rust
use std::sync::{mpsc, Arc, Mutex};

pub struct ComputePool {
    sender: mpsc::Sender<Box<dyn FnOnce() + Send>>,
    _workers: Vec<std::thread::JoinHandle<()>>,
}

impl ComputePool {
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = mpsc::channel::<Box<dyn FnOnce() + Send>>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(num_threads);

        for i in 0..num_threads {
            let receiver = receiver.clone();
            let handle = std::thread::Builder::new()
                .name(format!("velox-compute-{i}"))
                .spawn(move || loop {
                    let task = {
                        let rx = receiver.lock().unwrap();
                        rx.recv()
                    };
                    match task {
                        Ok(task) => task(),
                        Err(_) => break,
                    }
                })
                .expect("failed to spawn compute thread");
            workers.push(handle);
        }

        Self {
            sender,
            _workers: workers,
        }
    }

    pub fn spawn(&self, task: impl FnOnce() + Send + 'static) {
        let _ = self.sender.send(Box::new(task));
    }
}
```

**Step 5: Implement IoExecutor**

`crates/velox-runtime/src/executor/io_executor.rs`:
```rust
use std::future::Future;

pub struct IoExecutor {
    runtime: tokio::runtime::Runtime,
}

impl IoExecutor {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("velox-io")
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        Self { runtime }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.spawn(future);
    }

    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }
}

impl Default for IoExecutor {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 6: Implement DeliverQueue**

`crates/velox-runtime/src/executor/deliver.rs`:
```rust
use std::any::Any;
use std::collections::HashMap;
use std::sync::mpsc;

pub type TaskId = u64;
type DeliverCallback = Box<dyn FnOnce(Box<dyn Any + Send>)>;
type DeliverResult = (TaskId, Box<dyn Any + Send>);

pub struct DeliverQueue {
    next_id: TaskId,
    callbacks: HashMap<TaskId, DeliverCallback>,
    sender: mpsc::Sender<DeliverResult>,
    receiver: mpsc::Receiver<DeliverResult>,
}

impl DeliverQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            next_id: 0,
            callbacks: HashMap::new(),
            sender,
            receiver,
        }
    }

    pub fn register(&mut self, callback: impl FnOnce(Box<dyn Any + Send>) + 'static) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        self.callbacks.insert(id, Box::new(callback));
        id
    }

    pub fn sender(&self) -> mpsc::Sender<DeliverResult> {
        self.sender.clone()
    }

    pub fn send_result(&self, task_id: TaskId, result: Box<dyn Any + Send>) {
        let _ = self.sender.send((task_id, result));
    }

    pub fn flush(&mut self) {
        let pending: Vec<DeliverResult> = self.receiver.try_iter().collect();
        for (task_id, result) in pending {
            if let Some(callback) = self.callbacks.remove(&task_id) {
                callback(result);
            }
        }
    }
}

impl Default for DeliverQueue {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 7: Create executor module**

`crates/velox-runtime/src/executor/mod.rs`:
```rust
mod compute_pool;
mod deliver;
mod io_executor;
mod ui_queue;

pub use compute_pool::ComputePool;
pub use deliver::{DeliverQueue, TaskId};
pub use io_executor::IoExecutor;
pub use ui_queue::UiQueue;
```

**Step 8: Update lib.rs**

`crates/velox-runtime/src/lib.rs`:
```rust
mod cancellation;
pub mod executor;
mod frame_clock;

pub use cancellation::CancellationToken;
pub use frame_clock::FrameClock;
```

**Step 9: Add tokio dev-dependency for tests**

Add to `crates/velox-runtime/Cargo.toml` under `[dependencies]`:
```toml
[dev-dependencies]
tokio = { workspace = true, features = ["rt-multi-thread", "macros"] }
```

**Step 10: Run tests**

Run: `cargo test -p velox-runtime`
Expected: All tests pass.

**Step 11: Commit**

```bash
git add crates/velox-runtime/
git commit -m "feat(runtime): add UI queue, compute pool, IO executor, and deliver pattern"
```

---

### Task 7: velox-runtime — Runtime Struct + PowerPolicy

**Files:**
- Create: `crates/velox-runtime/src/power.rs`
- Create: `crates/velox-runtime/src/runtime.rs`
- Modify: `crates/velox-runtime/src/lib.rs`
- Create: `crates/velox-runtime/tests/runtime_tests.rs`

**Step 1: Write failing tests**

`crates/velox-runtime/tests/runtime_tests.rs`:
```rust
use velox_runtime::{PowerClass, PowerPolicy, Runtime};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn runtime_spawn_ui_runs_on_flush() {
    let mut runtime = Runtime::new();
    let (tx, rx) = mpsc::channel();

    runtime.spawn_ui(move || {
        tx.send(42).unwrap();
    });

    runtime.flush();
    assert_eq!(rx.recv_timeout(Duration::from_millis(100)).unwrap(), 42);
}

#[test]
fn runtime_spawn_compute_delivers_result() {
    let mut runtime = Runtime::new();
    let (tx, rx) = mpsc::channel();

    let task_id = runtime.spawn_compute(move || 42i32);
    runtime.register_deliver(task_id, move |result| {
        let value = *result.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });

    std::thread::sleep(Duration::from_millis(100));
    runtime.flush();

    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), 42);
}

#[test]
fn runtime_spawn_io_delivers_result() {
    let mut runtime = Runtime::new();
    let (tx, rx) = mpsc::channel();

    let task_id = runtime.spawn_io(async { 99i32 });
    runtime.register_deliver(task_id, move |result| {
        let value = *result.downcast::<i32>().unwrap();
        tx.send(value).unwrap();
    });

    std::thread::sleep(Duration::from_millis(100));
    runtime.flush();

    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), 99);
}

#[test]
fn power_policy_gates_decorative_on_battery() {
    let policy = PowerPolicy::Saving;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(!policy.should_run(PowerClass::Decorative));
    assert!(!policy.should_run(PowerClass::Background));
}

#[test]
fn power_policy_adaptive_allows_all() {
    let policy = PowerPolicy::Adaptive;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(policy.should_run(PowerClass::Decorative));
    assert!(policy.should_run(PowerClass::Background));
}

#[test]
fn power_policy_performance_allows_all() {
    let policy = PowerPolicy::Performance;
    assert!(policy.should_run(PowerClass::Essential));
    assert!(policy.should_run(PowerClass::Interactive));
    assert!(policy.should_run(PowerClass::Decorative));
    assert!(policy.should_run(PowerClass::Background));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-runtime`
Expected: Compilation error.

**Step 3: Implement PowerPolicy**

`crates/velox-runtime/src/power.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerClass {
    Essential,
    Interactive,
    Decorative,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPolicy {
    Performance,
    Adaptive,
    Saving,
}

impl PowerPolicy {
    pub fn should_run(&self, class: PowerClass) -> bool {
        match self {
            PowerPolicy::Performance | PowerPolicy::Adaptive => true,
            PowerPolicy::Saving => matches!(class, PowerClass::Essential | PowerClass::Interactive),
        }
    }
}

impl Default for PowerPolicy {
    fn default() -> Self {
        PowerPolicy::Adaptive
    }
}
```

**Step 4: Implement Runtime**

`crates/velox-runtime/src/runtime.rs`:
```rust
use std::any::Any;
use std::future::Future;

use crate::executor::{ComputePool, DeliverQueue, IoExecutor, TaskId, UiQueue};
use crate::frame_clock::FrameClock;
use crate::power::PowerPolicy;

pub struct Runtime {
    pub(crate) ui_queue: UiQueue,
    pub(crate) compute_pool: ComputePool,
    pub(crate) io_executor: IoExecutor,
    pub(crate) deliver_queue: DeliverQueue,
    pub(crate) frame_clock: FrameClock,
    pub(crate) power_policy: PowerPolicy,
}

impl Runtime {
    pub fn new() -> Self {
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(2);

        Self {
            ui_queue: UiQueue::new(),
            compute_pool: ComputePool::new(num_cpus),
            io_executor: IoExecutor::new(),
            deliver_queue: DeliverQueue::new(),
            frame_clock: FrameClock::new(),
            power_policy: PowerPolicy::default(),
        }
    }

    pub fn with_power_policy(mut self, policy: PowerPolicy) -> Self {
        self.power_policy = policy;
        self
    }

    pub fn spawn_ui(&mut self, task: impl FnOnce() + 'static) {
        self.ui_queue.push(Box::new(task));
    }

    pub fn spawn_compute<T>(&mut self, work: impl FnOnce() -> T + Send + 'static) -> TaskId
    where
        T: Send + 'static,
    {
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();

        self.compute_pool.spawn(move || {
            let result = work();
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });

        task_id
    }

    pub fn spawn_io<T, F>(&mut self, future: F) -> TaskId
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let task_id = self.deliver_queue.register_placeholder();
        let sender = self.deliver_queue.sender();

        self.io_executor.spawn(async move {
            let result = future.await;
            let _ = sender.send((task_id, Box::new(result) as Box<dyn Any + Send>));
        });

        task_id
    }

    pub fn register_deliver(
        &mut self,
        task_id: TaskId,
        callback: impl FnOnce(Box<dyn Any + Send>) + 'static,
    ) {
        self.deliver_queue.register_for(task_id, callback);
    }

    pub fn flush(&mut self) {
        self.frame_clock.tick();
        self.ui_queue.flush();
        self.deliver_queue.flush();
    }

    pub fn frame_clock(&self) -> &FrameClock {
        &self.frame_clock
    }

    pub fn power_policy(&self) -> PowerPolicy {
        self.power_policy
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 5: Update DeliverQueue to support placeholder registration**

Add these methods to `crates/velox-runtime/src/executor/deliver.rs`:

```rust
    pub fn register_placeholder(&mut self) -> TaskId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn register_for(
        &mut self,
        task_id: TaskId,
        callback: impl FnOnce(Box<dyn Any + Send>) + 'static,
    ) {
        self.callbacks.insert(task_id, Box::new(callback));
    }
```

**Step 6: Update lib.rs**

`crates/velox-runtime/src/lib.rs`:
```rust
mod cancellation;
pub mod executor;
mod frame_clock;
mod power;
mod runtime;

pub use cancellation::CancellationToken;
pub use frame_clock::FrameClock;
pub use power::{PowerClass, PowerPolicy};
pub use runtime::Runtime;
```

**Step 7: Run tests**

Run: `cargo test -p velox-runtime`
Expected: All tests pass.

**Step 8: Commit**

```bash
git add crates/velox-runtime/
git commit -m "feat(runtime): add Runtime struct with spawn APIs and PowerPolicy"
```

---

### Task 8: velox-window — Window + WindowManager

**Files:**
- Create: `crates/velox-window/src/config.rs`
- Create: `crates/velox-window/src/window_id.rs`
- Create: `crates/velox-window/src/manager.rs`
- Modify: `crates/velox-window/src/lib.rs`
- Create: `crates/velox-window/tests/config_tests.rs`

**Step 1: Write failing tests**

`crates/velox-window/tests/config_tests.rs`:
```rust
use velox_window::WindowConfig;

#[test]
fn window_config_builder() {
    let config = WindowConfig::new("main")
        .title("Test Window")
        .size(1200, 800)
        .min_size(400, 300)
        .resizable(true);

    assert_eq!(config.id_label(), "main");
    assert_eq!(config.title(), "Test Window");
    assert_eq!(config.size(), (1200, 800));
    assert_eq!(config.min_size(), Some((400, 300)));
    assert!(config.resizable());
}

#[test]
fn window_config_defaults() {
    let config = WindowConfig::new("default");
    assert_eq!(config.title(), "Velox");
    assert_eq!(config.size(), (800, 600));
    assert_eq!(config.min_size(), None);
    assert!(config.resizable());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-window`
Expected: Compilation error.

**Step 3: Implement WindowConfig**

`crates/velox-window/src/config.rs`:
```rust
pub struct WindowConfig {
    label: String,
    title: String,
    width: u32,
    height: u32,
    min_width: Option<u32>,
    min_height: Option<u32>,
    max_width: Option<u32>,
    max_height: Option<u32>,
    resizable: bool,
    decorations: bool,
}

impl WindowConfig {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            title: "Velox".to_string(),
            width: 800,
            height: 600,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            resizable: true,
            decorations: true,
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }

    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_width = Some(width);
        self.max_height = Some(height);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub fn id_label(&self) -> &str {
        &self.label
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn min_size(&self) -> Option<(u32, u32)> {
        match (self.min_width, self.min_height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    pub fn max_size(&self) -> Option<(u32, u32)> {
        match (self.max_width, self.max_height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }

    pub fn resizable(&self) -> bool {
        self.resizable
    }

    pub fn decorations(&self) -> bool {
        self.decorations
    }
}
```

**Step 4: Implement WindowId**

`crates/velox-window/src/window_id.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(winit::window::WindowId);

impl WindowId {
    pub(crate) fn from_winit(id: winit::window::WindowId) -> Self {
        Self(id)
    }

    pub(crate) fn winit_id(&self) -> winit::window::WindowId {
        self.0
    }
}
```

**Step 5: Implement WindowManager**

`crates/velox-window/src/manager.rs`:
```rust
use std::collections::HashMap;

use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

use crate::config::WindowConfig;
use crate::window_id::WindowId;

pub struct ManagedWindow {
    window: Window,
    label: String,
}

impl ManagedWindow {
    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

pub struct WindowManager {
    windows: HashMap<WindowId, ManagedWindow>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        config: WindowConfig,
    ) -> Result<WindowId, winit::error::OsError> {
        let mut attrs = WindowAttributes::default()
            .with_title(config.title())
            .with_inner_size(LogicalSize::new(config.size().0, config.size().1))
            .with_resizable(config.resizable())
            .with_decorations(config.decorations());

        if let Some((w, h)) = config.min_size() {
            attrs = attrs.with_min_inner_size(LogicalSize::new(w, h));
        }

        if let Some((w, h)) = config.max_size() {
            attrs = attrs.with_max_inner_size(LogicalSize::new(w, h));
        }

        let window = event_loop.create_window(attrs)?;
        let id = WindowId::from_winit(window.id());

        self.windows.insert(
            id,
            ManagedWindow {
                window,
                label: config.id_label().to_string(),
            },
        );

        Ok(id)
    }

    pub fn close_window(&mut self, id: WindowId) {
        self.windows.remove(&id);
    }

    pub fn get_window(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.windows.get(&id)
    }

    pub fn find_by_winit_id(&self, id: winit::window::WindowId) -> Option<WindowId> {
        let wid = WindowId::from_winit(id);
        if self.windows.contains_key(&wid) {
            Some(wid)
        } else {
            None
        }
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn request_redraws(&self) {
        for managed in self.windows.values() {
            managed.window.request_redraw();
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 6: Update lib.rs**

`crates/velox-window/src/lib.rs`:
```rust
mod config;
mod manager;
mod window_id;

pub use config::WindowConfig;
pub use manager::{ManagedWindow, WindowManager};
pub use window_id::WindowId;
```

**Step 7: Run tests and build**

Run: `cargo test -p velox-window && cargo build -p velox-window`
Expected: Tests pass, build succeeds.

Note: `WindowManager::create_window` cannot be unit tested without a real event loop. The config tests validate the builder pattern. Full window creation is verified by the demo in Task 10.

**Step 8: Commit**

```bash
git add crates/velox-window/
git commit -m "feat(window): add WindowConfig, WindowId, and WindowManager"
```

---

### Task 9: velox-app — App Builder + ApplicationHandler

**Files:**
- Create: `crates/velox-app/src/app.rs`
- Create: `crates/velox-app/src/handler.rs`
- Modify: `crates/velox-app/src/lib.rs`
- Create: `crates/velox-app/tests/app_tests.rs`

**Step 1: Write failing tests**

`crates/velox-app/tests/app_tests.rs`:
```rust
use velox_app::App;
use velox_runtime::PowerPolicy;
use velox_window::WindowConfig;

#[test]
fn app_builder_basic() {
    let app = App::new()
        .name("Test App")
        .power_policy(PowerPolicy::Adaptive)
        .window(WindowConfig::new("main").title("Test").size(800, 600));

    assert_eq!(app.name(), "Test App");
    assert_eq!(app.window_configs().len(), 1);
}

#[test]
fn app_builder_multi_window() {
    let app = App::new()
        .name("Multi Window")
        .window(WindowConfig::new("main").title("Main"))
        .window(WindowConfig::new("inspector").title("Inspector").size(400, 600));

    assert_eq!(app.window_configs().len(), 2);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-app`
Expected: Compilation error.

**Step 3: Implement App builder**

`crates/velox-app/src/app.rs`:
```rust
use velox_runtime::PowerPolicy;
use velox_window::WindowConfig;

use crate::handler::VeloxHandler;

pub struct App {
    name: String,
    power_policy: PowerPolicy,
    window_configs: Vec<WindowConfig>,
}

impl App {
    pub fn new() -> Self {
        Self {
            name: "Velox App".to_string(),
            power_policy: PowerPolicy::default(),
            window_configs: Vec::new(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn power_policy(mut self, policy: PowerPolicy) -> Self {
        self.power_policy = policy;
        self
    }

    pub fn window(mut self, config: WindowConfig) -> Self {
        self.window_configs.push(config);
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn window_configs(&self) -> &[WindowConfig] {
        &self.window_configs
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = winit::event_loop::EventLoop::new()?;
        let mut handler = VeloxHandler::new(self.power_policy, self.window_configs);
        event_loop.run_app(&mut handler)?;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
```

Note: The `name` method is used both as a builder method (taking `&str`) and as an accessor. Rust resolves this because the builder takes `self` by value. However, there is a naming conflict. Rename the accessor:

Replace the accessor with:
```rust
    pub fn app_name(&self) -> &str {
        &self.name
    }
```

And update the test to use `app_name()`.

Actually, a better approach: the builder method consumes self, so there's no ambiguity as long as we don't call builder methods on a reference. But the test calls `app.name()` on a non-consumed App. This creates a conflict.

Fix: rename the builder setter to `with_name` or rename the accessor. Let's keep the builder as `name()` and remove the accessor — tests can check via window_configs instead.

Revised approach: remove `app_name()` and `name()` accessor. The name is internal. Update test:

`crates/velox-app/tests/app_tests.rs`:
```rust
use velox_app::App;
use velox_runtime::PowerPolicy;
use velox_window::WindowConfig;

#[test]
fn app_builder_single_window() {
    let app = App::new()
        .name("Test App")
        .power_policy(PowerPolicy::Adaptive)
        .window(WindowConfig::new("main").title("Test").size(800, 600));

    assert_eq!(app.window_configs().len(), 1);
}

#[test]
fn app_builder_multi_window() {
    let app = App::new()
        .name("Multi Window")
        .window(WindowConfig::new("main").title("Main"))
        .window(WindowConfig::new("inspector").title("Inspector").size(400, 600));

    assert_eq!(app.window_configs().len(), 2);
}
```

**Step 4: Implement VeloxHandler (ApplicationHandler)**

`crates/velox-app/src/handler.rs`:
```rust
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId as WinitWindowId;

use velox_runtime::{PowerPolicy, Runtime};
use velox_window::{WindowConfig, WindowManager};

pub(crate) struct VeloxHandler {
    runtime: Runtime,
    window_manager: WindowManager,
    pending_windows: Vec<WindowConfig>,
    initialized: bool,
}

impl VeloxHandler {
    pub(crate) fn new(power_policy: PowerPolicy, window_configs: Vec<WindowConfig>) -> Self {
        Self {
            runtime: Runtime::new().with_power_policy(power_policy),
            window_manager: WindowManager::new(),
            pending_windows: window_configs,
            initialized: false,
        }
    }
}

impl ApplicationHandler for VeloxHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let configs = std::mem::take(&mut self.pending_windows);
        for config in configs {
            match self.window_manager.create_window(event_loop, config) {
                Ok(id) => {
                    eprintln!("[velox] Window created: {id:?}");
                }
                Err(err) => {
                    eprintln!("[velox] Failed to create window: {err}");
                }
            }
        }

        if self.window_manager.is_empty() {
            eprintln!("[velox] No windows created, exiting");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WinitWindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(id) = self.window_manager.find_by_winit_id(window_id) {
                    eprintln!("[velox] Window closed: {id:?}");
                    self.window_manager.close_window(id);
                }
                if self.window_manager.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {}
            WindowEvent::Resized(size) => {
                eprintln!("[velox] Window resized: {}x{}", size.width, size.height);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.runtime.flush();
        self.window_manager.request_redraws();
    }
}
```

**Step 5: Update lib.rs**

`crates/velox-app/src/lib.rs`:
```rust
mod app;
mod handler;

pub use app::App;
```

**Step 6: Run tests and build**

Run: `cargo test -p velox-app && cargo build -p velox-app`
Expected: Tests pass, build succeeds.

**Step 7: Commit**

```bash
git add crates/velox-app/
git commit -m "feat(app): add App builder with ApplicationHandler integration"
```

---

### Task 10: velox Facade + Demo Example

**Files:**
- Modify: `crates/velox/src/lib.rs`
- Create: `examples/demo.rs`
- Modify: root `Cargo.toml` (add example)

**Step 1: Update facade crate**

`crates/velox/src/lib.rs`:
```rust
pub use velox_app as app;
pub use velox_platform as platform;
pub use velox_reactive as reactive;
pub use velox_runtime as runtime;
pub use velox_window as window;

pub mod prelude {
    pub use velox_app::App;
    pub use velox_reactive::{Batch, Computed, Event, Signal, Subscription, SubscriptionBag};
    pub use velox_runtime::{PowerClass, PowerPolicy};
    pub use velox_window::WindowConfig;
}
```

**Step 2: Create demo example**

`examples/demo.rs`:
```rust
use velox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Velox Demo")
        .power_policy(PowerPolicy::Adaptive)
        .window(
            WindowConfig::new("main")
                .title("Velox Demo — Main Window")
                .size(1200, 800)
                .min_size(400, 300),
        )
        .window(
            WindowConfig::new("inspector")
                .title("Velox Demo — Inspector")
                .size(400, 600),
        )
        .run()
}
```

**Step 3: Add example to workspace Cargo.toml**

Add to root `Cargo.toml`:
```toml
[[example]]
name = "demo"
path = "examples/demo.rs"

[dependencies]
velox = { path = "crates/velox" }
```

Note: Examples at the workspace root need the dependency declared at root level.

Alternative (simpler): put the example inside the `velox` crate:

Move example to `crates/velox/examples/demo.rs` and add to `crates/velox/Cargo.toml`:
```toml
[[example]]
name = "demo"
```

**Step 4: Build and run the demo**

Run: `cargo build`
Expected: Full workspace compiles.

Run: `cargo run --example demo -p velox`
Expected: Two windows open on macOS. Closing both exits the app. Console shows `[velox] Window created` and `[velox] Window closed` messages.

**Step 5: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests across all crates pass.

Run: `cargo clippy --workspace`
Expected: No warnings.

**Step 6: Commit**

```bash
git add crates/velox/ examples/
git commit -m "feat: add velox facade crate and multi-window demo"
```

---

## Post-Implementation Verification

After all tasks are complete, verify success criteria from the design doc:

1. `cargo build` compiles the full workspace ✓
2. `cargo test --workspace` passes for all crates ✓
3. Demo app opens windows on macOS, handles resize/close, shuts down cleanly ✓
4. `spawn_ui`, `spawn_io`, `spawn_compute` work with deliver pattern (runtime tests) ✓
5. Signals, Computed, Events work with subscription cleanup (reactive tests) ✓
6. PowerPolicy gates work by PowerClass (runtime tests) ✓
7. Multi-window creation and independent event routing works (demo) ✓
