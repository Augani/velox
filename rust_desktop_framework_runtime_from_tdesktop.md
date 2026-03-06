# Building a High-Performance Rust Desktop Framework and Runtime
## Lessons from Telegram Desktop

## Purpose

This document lays out a concrete design for a Rust desktop framework and runtime that can support large, complex, responsive desktop applications. It is based on studying the structure and performance patterns in Telegram Desktop (`tdesktop`), then translating the useful ideas into a Rust-native architecture.

The goal is not to clone Telegram Desktop, Qt, or Electron. The goal is to build a Rust framework that gives application developers:

- native desktop performance
- explicit control over memory, scheduling, and rendering
- strong architectural boundaries for large codebases
- APIs that make advanced applications easier to build, not just simple windows and forms

## What `tdesktop` Actually Teaches

The main lesson from `tdesktop` is not "use Qt". The main lesson is "treat performance as an architectural property".

Several patterns show up repeatedly in this repository:

### 1. Bootstrap is thin; policy lives in the runtime

`Telegram/SourceFiles/main.cpp` is intentionally tiny. It creates a launcher and exits through it. Startup policy, platform setup, DPI settings, update handling, debug switches, and process-wide behavior live in `core/launcher.*` and `core/application.*`, not in `main()`.

This is the right model for the Rust framework:

- keep `main.rs` small
- put lifecycle, startup, recovery, platform policy, and global services in a runtime crate
- make application code plug into the runtime, not replace it

### 2. The app is split into internal libraries, not a single monolith

`Telegram/CMakeLists.txt` builds the app by composing internal modules such as `lib_rpl`, `lib_crl`, `lib_base`, `lib_ui`, `lib_storage`, `td_mtproto`, `td_ui`, and generated code modules.

That matters more than the language. It means the product is separated into:

- scheduling/runtime primitives
- reactive/event primitives
- UI toolkit code
- storage and networking layers
- generated protocol/style/lang code

The Rust framework should mirror this separation with multiple crates, not one giant `framework` crate.

### 3. Global app state is structured around domain, session, and windows

`main/main_domain.h` and `main/main_session.h` show a clear ownership model:

- `Domain` owns accounts and global session switching
- `Session` owns user-scoped services and state
- `Application` owns process-level services and windows
- `Window::Controller` and `Window::SessionController` bridge state into actual windows

This is one of the strongest patterns in the codebase. Large desktop apps need a real object model for ownership and lifetime:

- process
- workspace/domain
- account/session
- window
- section/view/controller

Without this, apps devolve into callback soup or giant global stores.

### 4. UI is reactive internally, even though it is not written like React

The codebase is full of `rpl::producer`, `rpl::variable`, `rpl::event_stream`, `rpl::combine`, `rpl::merge`, and `rpl::on_next`. That is a strong signal that `tdesktop` relies on internal reactive flows to propagate state changes efficiently.

The important lesson is not to copy `rpl` exactly. The lesson is:

- model state changes as streams/signals
- subscribe close to where work is needed
- invalidate only affected UI
- avoid broad "re-render everything" behavior

### 5. Expensive UI work is visibility-driven

This is one of the most important performance ideas in the repository.

Examples:

- `history/history_widget.cpp` preloads history when the user is within a few screenfuls of the edges
- `history/history_inner_widget.cpp` tracks the visible area, paints only intersecting items, and unloads heavy parts outside a bounded range
- `dialogs/dialogs_inner_widget.cpp` preloads row data ahead of the viewport
- `data/data_session.cpp` explicitly registers and unloads heavy view parts

This should become a first-class Rust framework subsystem:

- viewport-aware lists
- range-based prefetch
- range-based heavy resource unloading
- visibility signals available to views and services

### 6. Heavy work is moved off the main thread, but results are applied on the main thread

`crl::async` and `crl::on_main` appear throughout the codebase. A good example is `ui/chat/chat_theme.cpp`, where background caching work is done asynchronously and then applied back on the UI thread.

The pattern is consistent:

- compute/decode/cache away from the main thread
- keep UI ownership on the main thread
- return results through explicit main-thread handoff

That is exactly the model a Rust desktop runtime should formalize.

### 7. Styling is compiled, typed, and theme-aware

`Telegram/cmake/td_ui.cmake` shows `.style` files being compiled into generated UI code. Theme and palette changes are then propagated reactively across the app.

This is a strong architectural choice:

- styles are not ad hoc strings
- tokens are typed
- theme changes are dynamic
- style definitions are organized centrally

The Rust equivalent should be typed design tokens plus generated or macro-expanded style definitions.

### 8. Power saving is built into the product model

`core/application.cpp` and many UI files show explicit power-saving behavior:

- battery saving influences runtime behavior
- animation flags gate expensive effects
- some background activity is paused when needed
- feature-specific power-saving flags exist for chat background, stickers, emoji, and effects

This is a major lesson. Efficient frameworks should not bolt on power policy later. Power and thermal behavior must be part of the runtime.

### 9. Platform boundaries are explicit

`platform/platform_launcher.h` dispatches to OS-specific launchers, and the tree is split into `platform/win`, `platform/mac`, and `platform/linux`.

That does not mean the framework should expose platform code everywhere. It means:

- keep a common cross-platform contract
- isolate platform-specific behavior in dedicated modules
- expose native capabilities through narrow interfaces

### 10. Code generation is used where schemas are stable and repetitive

`generate_scheme.cmake`, `generate_lang.cmake`, and `td_ui.cmake` show code generation for protocol schemas, language resources, and styles.

This is another major lesson: if the framework wants typed protocols, typed styles, or typed resources, code generation is a feature, not a smell.

## Design Goals for the Rust Framework

The framework should optimize for these goals:

1. Build large native desktop applications, not toy demos.
2. Keep the UI thread predictable and small.
3. Make invalidation, rendering, and background work explicit.
4. Make virtualized content the default for large lists and timelines.
5. Let teams scale codebases across many windows, sessions, and services.
6. Support custom-rendered controls, not only stock widgets.
7. Keep platform features accessible without leaking platform details everywhere.
8. Make startup, memory use, and battery behavior first-class concerns.

## What Not to Build

Do not build:

- a thin wrapper around web views
- a DOM clone with desktop branding
- a framework that hides the event loop completely
- a framework that forces every view update through a full tree diff
- an ECS-first UI architecture for line-of-business desktop apps
- a styling system based on runtime string parsing

`tdesktop` works because it is disciplined about ownership, rendering, and work scheduling. A Rust framework should preserve that discipline, not abstract it away.

## Proposed Crate Architecture

The framework should be a workspace of focused crates.

```text
crates/
  app/
  runtime/
  reactive/
  platform/
  window/
  scene/
  render/
  text/
  style/
  animation/
  list/
  media/
  storage/
  codegen/
  devtools/
```

### `app`

High-level application assembly:

- application builder
- process configuration
- app manifest
- startup hooks
- service registration

This crate should feel small. It is an integration point, not the place where all logic lives.

### `runtime`

The most important crate.

Responsibilities:

- main-thread event loop integration
- frame clock
- timer wheel
- main-thread task queue
- background task scheduling
- cancellation and task lifetime binding
- power policy
- shutdown sequencing

This is the Rust equivalent of the combined role played by launcher/application/runtime glue in `tdesktop`.

### `reactive`

Lightweight state propagation primitives:

- `Signal<T>`
- `Computed<T>`
- `Event<T>`
- `Store<T>`
- `Subscription`
- `Effect`

This should support:

- cheap current-value access
- cheap subscription
- explicit lifetime-bound subscriptions
- derived values
- batching

This is conceptually similar to the role `rpl` plays throughout `tdesktop`.

### `platform`

Cross-platform contract plus per-OS implementations:

- app activation/focus
- tray support
- native notifications
- menus
- global shortcuts
- file dialogs
- clipboard
- IME
- accessibility bridge
- power/battery signals
- drag and drop

Common traits live here. OS implementations stay in `platform::win`, `platform::mac`, and `platform::linux`.

### `window`

Window lifecycle and shell management:

- window creation and destruction
- DPI/scaling policy
- surface creation
- geometry persistence
- modal/layer coordination
- window state restore
- tabbed/separate window policies

This should mirror the clean `Application` -> `Window::Controller` -> `MainWindow` layering seen in `tdesktop`.

### `scene`

Retained UI scene and view lifecycle:

- view tree
- layout tree
- invalidation regions
- input routing
- hit testing
- visibility tracking
- focus graph
- overlay/layer stack

This should not be purely declarative or purely imperative. It should be a hybrid:

- declarative structure for composition
- retained nodes for efficient updates
- imperative paint hooks for custom controls

### `render`

Rendering backend abstraction:

- surfaces
- command recording
- clipping
- layers
- textures
- atlases
- image decode upload
- GPU/CPU fallback policy

Recommended backend direction:

- use a native window/event-loop layer such as `winit` for cross-platform event loop and window creation
- use a GPU renderer such as `wgpu` as the primary rendering foundation
- keep a software or alternate fallback path because desktop apps still hit broken drivers and remote/virtualized environments

This recommendation is consistent with the need for explicit control and cross-platform rendering, and with the operational lesson from `tdesktop` that graphics failures and fallbacks are real product concerns.

### `text`

Text is too important to bury inside the renderer.

Responsibilities:

- shaping
- bidi
- fallback fonts
- emoji
- selection
- text editing
- IME composition
- glyph caching
- paragraph measurement

This crate should expose high-level text services to controls, lists, editors, and rich text surfaces.

### `style`

Typed styling and theming:

- design tokens
- palette
- typography
- spacing
- corners
- elevation
- animation constants
- generated or macro-defined style structs

The framework should compile style definitions into typed Rust structures, not parse CSS-like strings at runtime.

### `animation`

Animation runtime:

- frame-synchronized interpolation
- springs and easings
- transition orchestration
- paused/suppressed state under power saving
- lifecycle-bound animation handles

The framework should let animations participate in product policy. For example:

- disable decorative animation under battery saving
- keep semantic transitions enabled if needed
- prevent background animations from running off-screen

### `list`

Built-in virtualization and viewport services:

- variable-height virtual list
- grid virtualization
- sticky headers
- viewport range expansion
- prefetch hooks
- heavy-part unload hooks
- anchor-based scroll restoration

This must be a core crate, not a demo widget.

### `media`

Media lifecycle management:

- image decode pipeline
- video surfaces
- streaming hooks
- audio control bridge
- thumbnail generation
- visibility-based pause/resume

### `storage`

Persistence and background data services:

- settings store
- cache store
- indexed document store
- SQLite integration
- write coalescing
- migrations
- background compaction

### `codegen`

Generators for:

- typed styles
- resources
- language bundles
- protocol schemas
- icon registries

### `devtools`

Framework introspection:

- frame timing
- invalidation rectangles
- layout cost
- list virtualization stats
- GPU upload stats
- live resource graph
- view hierarchy inspector

If the framework wants to stay fast over time, developers need to see where the cost is.

## Runtime Design

### Threading Model

The framework should use a strict threading model:

- UI objects live on the main thread
- rendering submission is coordinated from the main thread
- I/O tasks run on an async runtime
- CPU-heavy tasks run on a bounded compute pool
- results return through explicit main-thread scheduling

Recommended queues:

- `spawn_ui`: run on main thread soon
- `spawn_io`: async I/O runtime
- `spawn_compute`: CPU-bound worker pool
- `spawn_after`: delayed execution
- `spawn_idle`: opportunistic low-priority work

Every spawned task should support:

- cancellation
- ownership binding to window/view/session lifetime
- priority hints
- tracing labels

### Event Loop Integration

The runtime should own the system event loop. Application authors should not call it directly.

The core loop should look like:

1. collect platform/window/input events
2. route events to views/controllers
3. run state updates
4. flush batched reactive effects
5. recompute layout for dirty subtrees
6. generate render commands for dirty regions
7. submit/present
8. run idle/prefetch tasks if budget remains

Important rule:

- never allow application code to directly nest uncontrolled event loops

### Lifetime Model

The framework should make lifetime explicit:

- process lifetime
- app lifetime
- workspace/domain lifetime
- session lifetime
- window lifetime
- view lifetime
- task lifetime

Every subscription, timer, and background task should be attachable to one of these scopes and automatically canceled on drop.

This is the Rust equivalent of the lifetime discipline visible in `tdesktop` through controller ownership, session ownership, and `rpl::lifetime`.

### Rendering Model

Use a retained scene with explicit invalidation.

Do not do:

- full redraw because "the GPU is fast enough"
- entire-tree diffing on every state change

Do:

- dirty-region tracking
- subtree layout invalidation
- subtree paint invalidation
- cached display lists or render bundles where useful
- texture and glyph atlas reuse
- offscreen cache surfaces for expensive repeated visuals

The framework should treat render cost in three bands:

- cheap: text/color/border updates
- medium: relayout and partial redraw
- expensive: blur, video, shadows, filters, large image upload

The runtime should understand those bands so it can make good scheduling decisions.

### Visibility and Virtualization Model

This is where the framework should most strongly copy `tdesktop`.

Each virtualized surface should expose:

- visible range
- expanded working range
- prefetch range
- unload range

Example policy:

- render visible items only
- keep 1 to 2 screens of layout/cache ahead
- prefetch 2 to 4 screens ahead depending on velocity
- unload heavy media outside the expanded working range

The framework API should let views react to visibility:

- `on_visible`
- `on_hidden`
- `on_visible_range_changed`
- `on_prefetch_range_changed`

This allows:

- image preloading
- message/history/page prefetch
- media pause/resume
- heavy effect unloading

### Power and Thermal Policy

The runtime should have built-in power policy, not just an on/off battery flag.

Suggested model:

```rust
enum PowerClass {
    Essential,
    Interactive,
    Decorative,
    Background,
}
```

Each task, animation, and effect can declare a power class.

Then the runtime can apply policy such as:

- pause decorative animations on battery saving
- reduce background prefetch depth
- throttle thumbnail generation
- suppress nonessential effects in background windows
- lower frame-rate targets for hidden windows

This follows the spirit of `tdesktop`'s explicit power-saving flags.

### Resource and Cache Model

The framework should distinguish:

- logical resources
- decoded resources
- uploaded GPU resources
- visible heavy resources

Suggested cache layers:

1. source cache
2. decoded image/media cache
3. layout/text cache
4. GPU texture/glyph cache
5. view-local short-lived cache

The runtime should budget each layer separately and expose pressure signals.

## Application Model

Application authors need a strong architectural shape.

Recommended hierarchy:

```text
App
  Domain/Workspace
    Session
      Service
      WindowController
        NavigationController
          Scene/View
```

### `App`

Process-wide concerns:

- startup
- global services
- crash reporting
- global preferences
- environment detection

### `Domain` or `Workspace`

Top-level business context:

- account switching
- organization/workspace selection
- multi-project state
- cache partitioning

### `Session`

User-scoped runtime state:

- repositories
- network clients
- permissions
- feature state
- active services

### `WindowController`

Owns:

- window shell
- window-local navigation
- focus policies
- overlays and layers
- layout mode changes

### `NavigationController`

Owns section transitions, stacks, split panes, tabs, and modal detail routes.

This is important because large desktop apps usually need:

- sidebars
- stacked navigation
- split views
- detachable windows
- previews
- inspector panes

That logic should not live inside leaf views.

## Reactive State Model

The framework should provide a small but strong reactive model:

- `Signal<T>` for mutable observable state
- `Computed<T>` for derived state
- `Event<T>` for fire-and-forget events
- `Batch` for grouped updates

Rules:

- view state reads should be cheap
- subscriptions should be local
- derived computations should be memoized by dependency graph, not by ad hoc user code
- state changes should invalidate precise parts of the tree

Avoid forcing application authors to manually wire low-level observer graphs for ordinary state.

## Styling and Theme System

The style system should be partly compile time, partly runtime.

Compile-time layer:

- style declarations
- token structs
- icon references
- typography definitions
- spacing constants

Runtime layer:

- palette
- dark/light/system mode
- user accent selection
- dynamic contrast adjustments
- reduced motion

Recommended API shape:

```rust
style_sheet! {
    pub struct ButtonStyle {
        bg: Color,
        bg_hover: Color,
        bg_pressed: Color,
        fg: Color,
        radius: Radius,
        padding: Insets,
        text: TextStyle,
    }
}
```

The key idea from `tdesktop` is:

- styles are authored centrally
- palette changes propagate reactively
- views do not invent arbitrary styling logic at random

## Animation System

Animations should be integrated with the runtime, not implemented as detached timers inside widgets.

Requirements:

- frame-synchronized
- cancel on view destruction
- suspend/resume with visibility
- downgrade under power-saving
- expose transition progress to paint code

Provide:

- tween animations
- spring animations
- crossfade helpers
- geometry interpolation
- value-keyframe support

## Input, Focus, and Editing

Complex desktop apps live or die on input quality.

The framework should invest deeply in:

- keyboard shortcuts
- command routing
- text editing
- IME
- pointer capture
- drag and drop
- hover tracking
- focus traversal
- accessibility actions

This is an area where many Rust UI experiments are still weaker than mature desktop products, so it should be treated as a core subsystem.

## Accessibility

Accessibility should be mandatory in the framework core.

Recommended direction:

- define a semantic accessibility tree in the scene layer
- bridge it per platform through an accessibility abstraction such as AccessKit
- keep custom-rendered controls accessible by construction, not by optional patching later

If the framework renders its own widgets, it must also provide its own accessibility story.

## Suggested Rust Technology Direction

This section is intentionally pragmatic.

### Windowing and Event Loop

Use a low-level native event loop and windowing layer such as `winit`, then wrap it in the framework runtime. `winit` explicitly documents itself as a cross-platform window creation and event loop library, with the event loop created on the main thread and owned once per application.

That aligns well with the runtime model above.

### GPU Rendering

Use `wgpu` as the main low-level GPU abstraction if the goal is a modern Rust-native rendering stack. `wgpu` is cross-platform and targets Vulkan, Metal, Direct3D 12, and OpenGL-class backends.

However, do not couple the framework API directly to `wgpu`. Keep a backend boundary so that:

- a software fallback can exist
- a different renderer can be used for special platforms
- rendering internals can evolve without breaking app code

### Text

Use a dedicated advanced text subsystem. A library such as `cosmic-text` is a strong fit for shaping, font fallback, layout, rasterization, and editing-oriented text handling.

### Async I/O

Use a separate async runtime for I/O and timers. Tokio remains a reasonable choice for networking and background async services, but it should sit behind the framework runtime, not become the UI runtime itself.

The framework should bridge to the I/O runtime through explicit spawn APIs and main-thread handoff.

### External References

- `winit`: <https://rust-windowing.github.io/winit/winit/>
- `wgpu`: <https://wgpu.rs/>
- `AccessKit`: <https://accesskit.dev/>
- `Tokio`: <https://tokio.rs/>
- `cosmic-text`: <https://github.com/pop-os/cosmic-text>

## Example Public API Shape

This is the kind of application-facing API the framework should expose.

```rust
use framework::prelude::*;

fn main() -> Result<()> {
    App::new()
        .name("Studio")
        .theme(ThemeMode::System)
        .power_policy(PowerPolicy::Adaptive)
        .service(ChatRepository::new())
        .service(MediaRepository::new())
        .window(main_window)
        .run()
}

fn main_window(cx: &mut AppContext) -> Window {
    Window::new("main")
        .title("Studio")
        .controller(MainWindowController::new(cx.session()))
        .content(|cx| {
            Split::columns()
                .left(ChatSidebar::new(cx.session()))
                .center(ChatTimeline::new(cx.session()))
                .right(Inspector::new(cx.session()))
        })
}
```

Example state and task API:

```rust
struct ChatTimeline {
    messages: Signal<VirtualModel<MessageId>>,
}

impl View for ChatTimeline {
    fn build(&mut self, cx: &mut ViewContext<Self>) -> Node {
        VirtualList::new(self.messages.clone())
            .row_height(RowHeight::Variable)
            .prefetch_screens(3)
            .unload_screens(2)
            .on_visible_range_changed(|range, cx| {
                cx.spawn_io("history-prefetch", move |io| async move {
                    io.repo::<ChatRepository>()
                        .prefetch_messages(range.expand_by(3))
                        .await
                });
            })
            .row(|id, cx| MessageRow::new(id))
            .into_node()
    }
}
```

Example background work handoff:

```rust
cx.spawn_compute("theme-cache", move || {
    compute_background_cache(input)
})
.deliver(cx, |result, cx| {
    cx.state_mut().background_cache = result;
    cx.invalidate_paint();
});
```

Example power-aware animation:

```rust
Animation::new()
    .class(PowerClass::Decorative)
    .duration_ms(180)
    .ease(Ease::OutCubic)
    .bind(opacity_signal);
```

## Performance Contract the Framework Should Enforce

The framework should publish a clear performance contract:

### Main Thread

- no blocking I/O
- no long-running decode
- no unbounded layout walks
- no global full-tree repaints from local state changes

### Rendering

- dirty region tracking by default
- atlas reuse by default
- clip-aware drawing by default
- GPU uploads visible in diagnostics

### Lists and Timelines

- virtualization on by default for large datasets
- range prefetch on by default
- heavy resources unload automatically unless pinned

### Background Work

- all nontrivial decode/parse/index work off the UI thread
- all results reenter through main-thread scheduling
- all background tasks cancelable

### Memory

- bounded caches
- resource pressure hooks
- visibility-based unloading

## Roadmap

### Phase 1: Runtime and Window Shell

Build first:

- event loop integration
- window abstraction
- main-thread scheduler
- I/O and compute executors
- lifetime-bound tasks
- power policy

Do not start with fancy widgets.

### Phase 2: Scene, Layout, and Invalidation

Build:

- retained node tree
- layout invalidation
- paint invalidation
- hit testing
- focus routing
- overlay stack

### Phase 3: Text, Input, and Custom Painting

Build:

- text shaping/editing
- IME
- selection
- keyboard shortcuts
- painter API
- cached layers and images

### Phase 4: Virtualized Collections and Media

Build:

- variable-height virtual list
- grid virtualization
- image/media caching
- visibility hooks
- prefetch and unload policies

### Phase 5: Style Codegen and Theming

Build:

- style DSL or macro system
- token generation
- theme manager
- palette propagation

### Phase 6: Accessibility and Devtools

Build:

- semantic accessibility tree
- platform adapters
- inspector
- performance overlay
- invalidation visualizer

## Risks and Failure Modes

### 1. Over-abstracting too early

If the first version tries to look like a universal GUI framework for every use case, it will likely become slow and vague.

Start with the target class of apps:

- multi-pane native desktop apps
- long-lived sessions
- large lists and timelines
- rich text
- media-heavy UIs

### 2. Under-investing in text and IME

Many "fast" frameworks fail here and become unusable for real productivity apps.

### 3. No visibility-aware resource lifecycle

If views cannot cheaply unload heavy parts and prefetch ahead of the viewport, the framework will eventually struggle with large content.

### 4. Treating the renderer as the framework

Rendering matters, but the real value is the runtime contract around rendering, state, and task lifetimes.

### 5. Ignoring platform behavior

Notifications, tray, menus, IME, accessibility, file dialogs, and window activation behavior are not optional for desktop software.

## Bottom Line

If you want a Rust framework that helps people build "desktop-class" applications, the blueprint from `tdesktop` is this:

- thin bootstrap
- strong runtime core
- explicit ownership model
- reactive state propagation
- visibility-driven rendering and loading
- background work with main-thread handoff
- typed style system
- platform boundaries
- power-aware behavior

The framework should feel less like "a Rust React clone" and more like "a desktop application operating system in library form".

That is the standard required if the target is not just building windows, but building applications that stay fast and maintainable after years of feature growth.

## Appendix: `tdesktop` Files Worth Studying

- `Telegram/SourceFiles/main.cpp`
- `Telegram/SourceFiles/core/launcher.h`
- `Telegram/SourceFiles/core/launcher.cpp`
- `Telegram/SourceFiles/core/application.h`
- `Telegram/SourceFiles/core/application.cpp`
- `Telegram/SourceFiles/main/main_domain.h`
- `Telegram/SourceFiles/main/main_session.h`
- `Telegram/SourceFiles/window/window_controller.h`
- `Telegram/SourceFiles/window/window_controller.cpp`
- `Telegram/SourceFiles/window/window_session_controller.h`
- `Telegram/SourceFiles/window/main_window.h`
- `Telegram/SourceFiles/history/history_widget.cpp`
- `Telegram/SourceFiles/history/history_inner_widget.cpp`
- `Telegram/SourceFiles/dialogs/dialogs_inner_widget.cpp`
- `Telegram/SourceFiles/dialogs/dialogs_main_list.cpp`
- `Telegram/SourceFiles/data/data_session.cpp`
- `Telegram/SourceFiles/ui/chat/chat_theme.cpp`
- `Telegram/cmake/td_ui.cmake`
- `Telegram/cmake/generate_scheme.cmake`
- `Telegram/cmake/generate_lang.cmake`
