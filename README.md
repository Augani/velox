# Velox

A high-performance Rust desktop framework for building large, complex, responsive applications.

Velox provides a complete toolkit — from GPU rendering and text shaping to reactive state management and virtualized collections — designed for applications that demand native performance and explicit control over resources.

## Architecture

```
velox              Facade crate, prelude, re-exports
velox-app          App builder, winit ApplicationHandler
velox-reactive     Signal<T>, Computed<T>, Event<T>, batched updates
velox-runtime      FrameClock, thread pools (UI/IO/compute), power policy
velox-platform     Platform abstraction (power, clipboard, app lifecycle)
velox-window       Window config, multi-window management
velox-scene        Retained node tree, paint commands, hit testing, focus, overlays
velox-text         cosmic-text integration, editable text, selection, undo/redo
velox-render       wgpu GPU renderer with software fallback, glyph atlas
velox-style        Typed theme tokens, palette/theme manager, style DSL
velox-ui           Element primitives, Taffy layout, reconciler, reactive components
velox-list         Virtualized lists/grids, scroll anchoring, visibility callbacks
velox-animation    Frame-synced animations, power-aware transitions
velox-storage      Settings, cache, SQLite, migrations
velox-media        Image/video decode pipeline
velox-codegen      Style/resource generators
velox-devtools     Frame timing, layout inspector
```

## Key Design Principles

- **Runtime owns the event loop** — app code plugs into the runtime, never drives the loop directly
- **Strict threading model** — UI on main thread, I/O on async runtime, compute on bounded pool
- **Visibility-driven work** — render and load only what's visible, prefetch ahead, unload offscreen
- **Explicit invalidation** — dirty regions and subtree invalidation, not full tree diffs
- **Lifetime-bound resources** — tasks, subscriptions, and animations cancel on scope drop
- **Power-aware by default** — power class gates animations and background work

## Requirements

- Rust 1.85+ (edition 2024)
- GPU: wgpu-compatible (Vulkan, Metal, DX12) with automatic software fallback

## Quick Start

```bash
cargo build --workspace
cargo test --workspace
cargo run -p velox --example demo
```

## Threading Model

```rust
spawn_ui(...)       // Main thread, immediate
spawn_io(...)       // Async I/O runtime (tokio)
spawn_compute(...)  // CPU-bound worker pool
spawn_after(...)    // Delayed execution
spawn_idle(...)     // Low-priority opportunistic
```

All spawned tasks support cancellation, ownership binding, and priority hints.

## UI Elements

Velox provides a declarative element system with Taffy flexbox layout:

```rust
use velox_ui::*;

div()
    .flex_row()
    .gap(px(8.0))
    .p(px(16.0))
    .bg(Color::rgb(30, 30, 30))
    .hover(|s| s.bg(Color::rgb(50, 50, 50)))
    .child(text("Hello, Velox").text_lg().text_color(Color::WHITE))
    .child(
        input()
            .placeholder("Type here...")
            .on_change(|text| println!("{text}"))
    )
```

Elements include `div`, `text`, `input`, `list` (virtualized), `overlay`/`modal`, `canvas`, `img`, and `svg`.

## Reactive State

```rust
use velox_reactive::*;

let count = Signal::new(0);
let doubled = Computed::new({
    let count = count.clone();
    move || count.get() * 2
});

count.set(5);
assert_eq!(doubled.get(), 10);
```

Components automatically track signal dependencies and re-render when they change.

## License

MIT
