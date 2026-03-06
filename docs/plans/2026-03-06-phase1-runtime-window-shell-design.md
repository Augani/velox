# Phase 1: Runtime and Window Shell — Design

## Overview

Phase 1 establishes the foundation: a Rust workspace with 6 crates that provide the runtime core, reactive state, platform abstraction, window management, and application assembly. No rendering, no scene graph, no widgets. The goal is a solid runtime that opens windows, schedules tasks, and shuts down cleanly.

## Workspace Structure

```
velox/
  Cargo.toml              # workspace root
  crates/
    velox/                 # re-export facade crate (velox::prelude::*)
    velox-runtime/         # event loop, schedulers, task system, power policy
    velox-reactive/        # Signal, Computed, Event, Subscription
    velox-platform/        # platform traits + macOS impl
    velox-window/          # window lifecycle, DPI, surface
    velox-app/             # application builder, startup, service registry
```

### Dependency Flow

```
velox-app → velox-window → velox-platform
         → velox-runtime
         → velox-reactive
```

`velox` is the facade — users depend on `velox` and get a curated `prelude`. Internal crates depend on each other directly.

## Crate: `velox-runtime`

The most important crate. Owns the main thread and all task scheduling.

### Components

- **MainLoop** — wraps winit's event loop, owns the frame clock and timer wheel. Application code never touches winit directly.
- **Executor** — four task queues:
  - `spawn_ui(task)` — runs on main thread next frame
  - `spawn_io(future)` — runs on tokio runtime (networking, file I/O)
  - `spawn_compute(closure)` — runs on a bounded thread pool
  - `spawn_after(duration, task)` — delayed main-thread execution
- **TaskHandle** — returned from every spawn. Supports cancellation and lifetime binding. When a scope (window, view) drops, its bound tasks cancel automatically.
- **PowerPolicy** — `PowerClass` enum (Essential, Interactive, Decorative, Background). The runtime queries battery state via platform crate and gates work accordingly. Phase 1 is the skeleton — animations/effects come later.
- **FrameClock** — tracks frame timing, exposes `delta_time` and `frame_count`.

### Deliver Pattern

Results from `spawn_io` and `spawn_compute` return to the main thread via `.deliver(cx, callback)` — never direct shared state mutation.

```rust
let handle = cx.spawn_compute("decode-image", move || {
    decode_png(bytes)
});
handle.deliver(cx, |result, cx| {
    cx.set_state(|s| s.image = Some(result));
});
```

## Crate: `velox-reactive`

Lightweight reactive primitives. No framework dependency — pure Rust, no platform or UI knowledge.

### Primitives

- **`Signal<T>`** — mutable observable value. Cheap read, cheap subscribe. Clone gives a shared handle.
- **`Computed<T>`** — derived from signals. Memoized, lazy — only recomputes when dependencies change.
- **`Event<T>`** — fire-and-forget stream. No current value, just notifications.
- **`Subscription`** — drop cancels. Can be bound to a `SubscriptionBag` for batch cleanup.
- **`Batch`** — groups signal updates, defers notifications until batch completes.

### Design Constraints

- No global runtime. Signals work standalone and are testable in isolation.
- The runtime's frame flush drains batched reactive effects before layout/render. The reactive crate itself doesn't know about frames.

```rust
let count = Signal::new(0);
let doubled = Computed::new({
    let count = count.clone();
    move || count.get() * 2
});

let _sub = doubled.subscribe(|val| {
    println!("doubled: {val}");
});

count.set(5); // prints "doubled: 10"
```

## Crate: `velox-platform`

Cross-platform traits with macOS as the first implementation.

### Traits

- **`PlatformApp`** — app activation, dock icon, global menu bar
- **`PlatformWindow`** — native window handle, title bar customization, fullscreen
- **`PlatformPower`** — battery state, thermal state, power source changes
- **`PlatformClipboard`** — read/write text

### Structure

```
velox-platform/
  src/
    lib.rs          # trait definitions
    mac/            # macOS impl via objc2
    win/            # stubs
    linux/          # stubs
```

macOS uses `objc2` for features winit doesn't cover (dock, menu bar, power state). Windows and Linux are stubbed (compile, return defaults).

## Crate: `velox-window`

- **`Window`** — wraps winit window + platform window handle. Owns surface creation (raw window handle only — rendering comes later).
- **`WindowConfig`** — title, size, min/max size, position, resizable, decorations, DPI policy.
- **`WindowManager`** — tracks open windows, routes events to correct window. Multi-window from day one.
- **`WindowId`** — typed wrapper around winit's window ID.

No rendering, no views. Windows open, receive events, and close cleanly.

## Crate: `velox-app`

- **`App::new()`** — builder pattern. Registers services, sets power policy, defines windows.
- **`AppContext`** — passed to callbacks. Access to runtime, window manager, service registry, power state.
- **`ServiceRegistry`** — type-map of `Arc<dyn Any + Send + Sync>`. Services registered at startup, accessed via `cx.service::<T>()`.

```rust
fn main() -> Result<()> {
    App::new()
        .name("Demo")
        .power_policy(PowerPolicy::Adaptive)
        .service(MyService::new())
        .window(|cx| {
            WindowConfig::new("main")
                .title("Velox Demo")
                .size(1200, 800)
        })
        .run()
}
```

`App::run()` initializes the runtime, creates windows, enters the main loop, handles shutdown.

## Technology

- **winit** — cross-platform windowing and event loop
- **tokio** — async I/O runtime (behind abstraction)
- **objc2** — macOS platform features
- **Target:** macOS first, platform traits ready for Windows/Linux

## Success Criteria

Phase 1 is done when:
1. `cargo build` compiles the full workspace
2. `cargo test` passes for all crates
3. A demo app opens a window on macOS, handles resize/close events, and shuts down cleanly
4. `spawn_ui`, `spawn_io`, `spawn_compute`, `spawn_after` all work with cancellation
5. Signals, Computed, and Events work with subscription cleanup
6. Power state is queryable on macOS
7. Multi-window creation and independent event routing works
