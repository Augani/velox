# Deferred Items Design — Velox Framework

## Overview

8 deferred items from the original design doc, implemented in 3 waves by priority.

## Wave A: Essential

### A1. AccessKit Bridge

Bridge existing `AccessibilityTreeSnapshot` to OS accessibility via AccessKit.

- Add `accesskit`, `accesskit_winit` workspace deps
- `AccessibilityBridge` in velox-platform converts `AccessibilityTreeSnapshot` -> `accesskit::TreeUpdate`
- Map `AccessibilityRole` -> `accesskit::Role`
- In `VeloxHandler`, after layout+paint, diff accessibility tree and push updates via `accesskit_winit::Adapter`
- Handle incoming AccessKit actions (focus, click, set_value) by routing to scene event handlers

### A2. IME Composition

Handle winit `WindowEvent::Ime` for CJK and international text input.

- `CompositionState { preedit_text, cursor_range }` in velox-text
- `ImeEvent` enum (Preedit/Commit/Enabled/Disabled) in velox-scene events
- `handle_ime()` method on `EventHandler` trait
- `EditableText` renders inline preedit with underline, commits insert final text
- `VeloxHandler` wires `WindowEvent::Ime` to focused node, manages `set_ime_allowed`/`set_ime_cursor_area`

### A3. Modal Coordination

Extend `OverlayStack` with modal semantics.

- `ModalConfig { backdrop_dismisses, trap_focus, blocks_parent }`
- `push_modal(config) -> OverlayId`
- Focus trapping within modal tree, backdrop click handling
- `FocusState::push_scope/pop_scope` for restricted focus traversal
- `WindowManager::set_modal_for(child, parent)` for cross-window blocking

## Wave B: High Impact

### B1. Drag and Drop

Platform DnD + intra-app drag via scene abstractions.

- `DragPayload` enum: Files, Text, Custom
- `DropTarget` trait: `accepts()`, `on_drop()`
- `set_drop_target(node_id, impl DropTarget)` on NodeTree
- VeloxHandler handles winit file drop events, hit tests for drop targets
- Intra-app: `Scene::start_drag(source, payload)` with DragState machine + drag overlay

### B2. Layer Compositing

Offscreen render targets for opacity groups, blur, shadow.

- `RenderLayer` wraps wgpu texture for offscreen rendering
- `PaintCommand::PushLayer { opacity, blend_mode }` / `PopLayer`
- `BlendMode`: Normal, Multiply, Screen, Overlay
- `PaintCommand::BoxShadow { rect, color, blur_radius, offset, spread }`
- Gaussian blur via two-pass fragment shader with ping-pong textures
- Phase 1: opacity layers only; Phase 2: blur

### B3. BiDi Text

RTL cursor movement and selection on top of cosmic-text's BiDi shaping.

- Use `LayoutRun::rtl` to determine visual cursor direction
- `CursorDirection::Left/Right` follows visual order in RTL runs
- Selection stored as logical offsets, rendered as per-run visual highlight rects
- `TextBuffer::paragraph_direction()` based on first strong character

## Wave C: Polish

### C1. GPU/CPU Fallback

Software rendering via softbuffer + tiny-skia when wgpu unavailable.

- `RenderBackend` enum: Gpu(Renderer) | Software(SoftwareRenderer)
- `SoftwareRenderer` interprets paint commands, rasterizes to CPU buffer
- Fallback triggered when `GpuContext::new()` fails
- Text via existing `GlyphRasterizer` blitted to framebuffer
- Targets 30fps simple UIs, skip complex effects

### C2. Live Resource Graph

Queryable snapshot of framework-managed resources for devtools.

- `ResourceNode` enum: GlyphAtlas, TexturePool, CacheStore, AnimationPool with metrics
- `ResourceGraph::snapshot()` collects state from subsystems
- `ResourceGraph::diff()` detects significant changes
- Data collection only; visualization deferred

## Dependencies

```
Wave A: A1, A2, A3 are independent
Wave B: B1, B2, B3 are independent (after A)
Wave C: C1, C2 are independent (after B)
```

## Workspace Deps

```toml
accesskit = "0.17"
accesskit_winit = "0.23"
softbuffer = "0.4"
tiny-skia = "0.11"
```
