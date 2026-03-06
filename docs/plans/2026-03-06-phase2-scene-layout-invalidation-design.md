# Phase 2: Scene, Layout, and Invalidation — Design

## Overview

Phase 2 adds a new `velox-scene` crate that provides the retained UI node tree, layout system, paint invalidation, hit testing, focus state, and overlay stack. No actual rendering — paint commands are recorded for a future renderer. The goal is a testable scene architecture that integrates into the existing runtime and window system.

## Crate: `velox-scene`

**Dependencies:** `velox-reactive`, `velox-runtime`
**Consumers:** `velox-app` wires scenes into window events

### Dependency Flow (updated)

```
velox-app → velox-scene → velox-reactive
         → velox-window    velox-runtime
         → velox-runtime
         → velox-reactive
```

## Node Tree

Arena-backed tree using `slotmap` for stable node IDs that survive insertions and removals.

### Data Structures

```rust
pub struct NodeId(slotmap::DefaultKey);

pub struct NodeData {
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    rect: Rect,
    visible: bool,
    layout_dirty: bool,
    paint_dirty: bool,
    hit_test_transparent: bool,
    layout: Option<Box<dyn Layout>>,
    painter: Option<Box<dyn Painter>>,
}

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct Size {
    pub width: f32,
    pub height: f32,
}

pub struct Point {
    pub x: f32,
    pub y: f32,
}
```

### NodeTree

```rust
pub struct NodeTree {
    nodes: SlotMap<DefaultKey, NodeData>,
    root: Option<NodeId>,
}
```

Operations:
- `insert(parent: Option<NodeId>) -> NodeId` — create node, append to parent's children
- `remove(id: NodeId)` — remove node and all descendants
- `reparent(id: NodeId, new_parent: NodeId)` — move node to new parent
- `children(id: NodeId) -> &[NodeId]`
- `parent(id: NodeId) -> Option<NodeId>`
- `set_rect(id: NodeId, rect: Rect)` — sets geometry, marks paint dirty
- `set_visible(id: NodeId, visible: bool)` — marks paint dirty
- `mark_layout_dirty(id: NodeId)` — propagates up to root
- `mark_paint_dirty(id: NodeId)`

## Layout

### Design

Manual layout by default — parent sets child geometry via `set_rect()`. Optional layout helpers implement the `Layout` trait for common patterns.

Layout invalidation: adding/removing children or calling `mark_layout_dirty()` flags the node. Layout pass runs top-down, only visiting dirty subtrees.

### Trait

```rust
pub trait Layout {
    fn compute(&self, parent_size: Size, children: &[NodeId], tree: &mut NodeTree);
}
```

### Built-in Helpers

**StackLayout** — distributes children along an axis with spacing:

```rust
pub enum Direction {
    Horizontal,
    Vertical,
}

pub struct StackLayout {
    pub direction: Direction,
    pub spacing: f32,
}
```

**PaddingLayout** — insets the single child within parent bounds:

```rust
pub struct PaddingLayout {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}
```

### Layout Pass

Top-down traversal from root. For each dirty node:
1. If node has a `Layout`, call `layout.compute(parent_size, children, tree)`
2. Clear `layout_dirty` flag
3. Recurse into children

Clean subtrees are skipped entirely.

## Invalidation & Paint Commands

### Dirty Flags

Two independent flags per node:
- `layout_dirty` — needs layout recomputation
- `paint_dirty` — needs repaint

Setting geometry, changing visibility, or calling `invalidate_paint()` marks paint dirty. Adding/removing children marks layout dirty.

### Paint Commands

```rust
pub enum PaintCommand {
    FillRect { rect: Rect, color: Color },
    StrokeRect { rect: Rect, color: Color, width: f32 },
    PushClip(Rect),
    PopClip,
    Custom(Box<dyn Any>),
}

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct CommandList {
    commands: Vec<PaintCommand>,
}
```

### Painter Trait

```rust
pub trait Painter {
    fn paint(&self, rect: Rect, commands: &mut CommandList);
}
```

### Paint Pass

Top-down traversal. For each node:
1. Skip if not visible
2. `PushClip(node.rect)`
3. If node has a `Painter`, call `painter.paint(rect, commands)`
4. Recurse into children
5. `PopClip`
6. Clear `paint_dirty` flag

The `CommandList` is collected per-window, cleared each frame. Ready for a real renderer in a later phase.

## Hit Testing

Top-down traversal. Check overlay stack first (topmost to bottommost), then main tree.

```rust
impl Scene {
    pub fn hit_test(&self, point: Point) -> Option<NodeId>;
}
```

Algorithm:
1. For each overlay (topmost first), run hit test on its tree
2. If found, return immediately
3. Otherwise, hit test the main tree
4. At each node: check bounds, skip if `hit_test_transparent`, descend into children in reverse order (last child = topmost)
5. Return deepest matching node

## Focus

Minimal focus state per scene:

```rust
pub struct FocusState {
    focused: Option<NodeId>,
    change_event: Event<FocusChange>,
}

pub struct FocusChange {
    pub lost: Option<NodeId>,
    pub gained: Option<NodeId>,
}
```

API:
- `request_focus(node_id: NodeId)` — sets focus, emits change event
- `release_focus()` — clears focus, emits change event
- `focused() -> Option<NodeId>`
- `on_focus_change() -> Subscription` — subscribe to focus changes

No automatic tab traversal — that comes in Phase 3 with keyboard/input work.

## Overlay Stack

```rust
pub struct OverlayEntry {
    id: OverlayId,
    tree: NodeTree,
}

pub struct OverlayStack {
    overlays: Vec<OverlayEntry>,
    next_id: u64,
}
```

API:
- `push_overlay() -> OverlayId` — create new overlay with empty tree, returns ID
- `overlay_tree(id: OverlayId) -> &mut NodeTree` — access overlay's node tree
- `pop_overlay(id: OverlayId)` — remove specific overlay
- `dismiss_all()` — clear all overlays
- `is_empty() -> bool`

Overlays render on top of the main tree. Hit testing checks overlays first.

## Scene (per-window)

```rust
pub struct Scene {
    tree: NodeTree,
    overlay_stack: OverlayStack,
    focus: FocusState,
    command_list: CommandList,
}
```

API:
- `tree() -> &NodeTree` / `tree_mut() -> &mut NodeTree`
- `overlay_stack() -> &OverlayStack` / `overlay_stack_mut() -> &mut OverlayStack`
- `focus() -> &FocusState` / `focus_mut() -> &mut FocusState`
- `layout()` — run layout pass on dirty subtrees
- `paint()` — run paint pass, populate command list
- `hit_test(point: Point) -> Option<NodeId>`
- `commands() -> &CommandList`

## Integration with velox-app

`VeloxHandler` gains a `HashMap<WindowId, Scene>`:

- `resumed()` — create a `Scene` for each window
- `RedrawRequested` — call `scene.layout()` then `scene.paint()`
- Pointer events — call `scene.hit_test(point)`
- `about_to_wait()` — after runtime flush, request redraw for windows with dirty scenes

## Technology

- **slotmap** — arena allocation with stable keys for node IDs
- No new external dependencies beyond slotmap

## Success Criteria

Phase 2 is done when:
1. `velox-scene` compiles and all tests pass
2. NodeTree supports insert, remove, reparent, traversal
3. Layout pass computes geometry for dirty subtrees only
4. StackLayout and PaddingLayout produce correct child positions
5. Paint pass emits correct CommandList, skipping clean nodes
6. Hit testing returns correct NodeId by point
7. FocusState tracks focus with change events
8. OverlayStack routes hit tests to topmost overlay first
9. Scene integrates into VeloxHandler event loop
