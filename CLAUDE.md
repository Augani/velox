# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Velox is a high-performance Rust desktop framework and runtime for building large, complex, responsive desktop applications. The design is informed by Telegram Desktop's architecture patterns, translated into idiomatic Rust.

**Status:** Phase 2 complete (scene + layout + invalidation). See `rust_desktop_framework_runtime_from_tdesktop.md` for the full design document.

## Build Commands

```bash
cargo build                          # Build all crates
cargo test --workspace               # Run all tests
cargo test -p velox-runtime          # Test specific crate
cargo clippy --workspace             # Lint
cargo run -p velox --example demo    # Run multi-window demo
```

## Crate Architecture

### Implemented (Phase 1 + Phase 2)

```
crates/
  velox/          # Facade crate with prelude, re-exports all sub-crates
  velox-app/      # App builder, VeloxHandler (winit ApplicationHandler)
  velox-reactive/ # Signal<T> (auto-tracking), Computed<T>, Event<T>, Batch
  velox-runtime/  # FrameClock, CancellationToken, UiQueue, ComputePool, IoExecutor, DeliverQueue, Runtime, PowerPolicy
  velox-platform/ # PlatformPower/App/Clipboard traits + StubPlatform
  velox-window/   # WindowConfig, WindowId, ManagedWindow, WindowManager
  velox-scene/    # Retained node tree, layout, paint commands, hit testing, focus, overlays
```

### Planned (Phase 3+)

```
crates/
  velox-render/    # GPU abstraction, surfaces, atlases, fallback paths
  velox-text/      # Shaping, bidi, IME, selection, glyph caching
  velox-style/     # Typed design tokens, theming, palette
  velox-animation/ # Frame-synced animations, power-aware transitions
  velox-list/      # Virtualized lists/grids, prefetch, visibility hooks
  velox-media/     # Image/video decode pipeline, visibility-based pause
  velox-storage/   # Settings, cache, SQLite, migrations
  velox-codegen/   # Style/resource/protocol generators
  velox-devtools/  # Frame timing, layout cost, inspector
```

## Core Architectural Principles

1. **Runtime owns the event loop** - App code plugs into runtime, never calls event loop directly
2. **Strict threading model** - UI on main thread, I/O on async runtime, compute on bounded pool, explicit main-thread handoff for results
3. **Visibility-driven work** - Render/load only visible items, prefetch ahead, unload heavy resources outside viewport
4. **Explicit invalidation** - Dirty regions, not full redraws; subtree invalidation, not tree diffs
5. **Lifetime-bound resources** - Tasks, subscriptions, animations cancel on scope drop (view/window/session)
6. **Power-aware by default** - PowerClass (Essential/Interactive/Decorative/Background) gates animations and background work

## Threading Model

```rust
spawn_ui(...)      // Main thread, soon
spawn_io(...)      // Async I/O runtime
spawn_compute(...) // CPU-bound worker pool
spawn_after(...)   // Delayed execution
spawn_idle(...)    // Low-priority opportunistic
```

All spawned tasks support cancellation, ownership binding, priority hints.

## Technology Stack

- **Windowing:** winit
- **GPU:** wgpu (with software fallback)
- **Text:** cosmic-text
- **Async I/O:** tokio (behind runtime abstraction)
- **Accessibility:** AccessKit

## Implementation Order

1. Runtime + window shell (event loop, schedulers, lifetime management) -- DONE
2. Scene + layout + invalidation -- DONE
3. Text + input + custom painting
4. Virtualized collections + media
5. Style codegen + theming
6. Accessibility + devtools

## Key Patterns to Follow

- Keep `main.rs` thin; policy lives in runtime
- Split into focused crates, not a monolith
- State hierarchy: App > Domain/Workspace > Session > Window > View
- Reactive state: Signal/Computed for propagation, not manual wiring
- Styles are typed structs, not parsed strings
- Views respond to visibility: `on_visible`, `on_hidden`, `on_prefetch_range_changed`

## Performance Contract

- No blocking I/O on main thread
- No unbounded layout walks
- Virtualization default for large datasets
- All decode/parse work off UI thread
- Bounded caches with pressure signals
