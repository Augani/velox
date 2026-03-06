# Phase 2: Scene, Layout, and Invalidation — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a `velox-scene` crate providing a retained node tree, layout system, paint command recording, hit testing, focus state, and overlay stack.

**Architecture:** Arena-backed node tree using `slotmap`. Nodes hold geometry, dirty flags, optional layout helpers, and paint callbacks. A `Scene` struct per window orchestrates layout passes, paint command collection, hit testing, focus, and overlays. Integration into `velox-app`'s `VeloxHandler` connects the scene to the window event loop.

**Tech Stack:** Rust (edition 2024), slotmap 1, velox-reactive (Event for focus changes)

---

## Context

### Existing Crates (Phase 1)

- `velox-reactive` — Signal, Computed, Event, Batch, Subscription, SubscriptionBag
- `velox-runtime` — Runtime, FrameClock, PowerPolicy, task executors
- `velox-platform` — platform traits with stub impls
- `velox-window` — WindowConfig, WindowId, WindowManager, ManagedWindow
- `velox-app` — App builder, VeloxHandler (ApplicationHandler impl)
- `velox` — facade re-exports + prelude

### Key Files to Understand

- `crates/velox-app/src/handler.rs` — VeloxHandler: owns Runtime + WindowManager, handles winit events
- `crates/velox-window/src/manager.rs` — WindowManager: HashMap<WindowId, ManagedWindow>
- `crates/velox-window/src/window_id.rs` — WindowId newtype wrapping winit::window::WindowId
- `crates/velox-reactive/src/event.rs` — Event<T>: fire-and-forget stream with subscribe/emit
- `crates/velox-reactive/src/subscription.rs` — Subscription with SubscriptionFlag (Rc<Cell<bool>>)

### Design Doc

See `docs/plans/2026-03-06-phase2-scene-layout-invalidation-design.md` for the full design.

---

## Task 1: Scaffold `velox-scene` Crate

**Files:**
- Create: `crates/velox-scene/Cargo.toml`
- Create: `crates/velox-scene/src/lib.rs`
- Create: `crates/velox-scene/src/geometry.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Create crate directory and Cargo.toml**

```toml
# crates/velox-scene/Cargo.toml
[package]
name = "velox-scene"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
velox-reactive = { workspace = true }
slotmap = "1"
```

**Step 2: Create geometry types in `src/geometry.rs`**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
```

**Step 3: Create `src/lib.rs`**

```rust
mod geometry;

pub use geometry::{Point, Rect, Size};
```

**Step 4: Add to workspace**

In the root `Cargo.toml`, add `"crates/velox-scene"` to the `members` array and add the workspace dependency:

```toml
[workspace]
members = [
    "crates/velox",
    "crates/velox-reactive",
    "crates/velox-platform",
    "crates/velox-runtime",
    "crates/velox-window",
    "crates/velox-app",
    "crates/velox-scene",
]

[workspace.dependencies]
# ... existing entries ...
velox-scene = { path = "crates/velox-scene" }
slotmap = "1"
```

**Step 5: Verify it compiles**

Run: `cargo build -p velox-scene`
Expected: compiles with no errors

**Step 6: Commit**

```bash
git add crates/velox-scene/ Cargo.toml Cargo.lock
git commit -m "feat(scene): scaffold velox-scene crate with geometry types"
```

---

## Task 2: NodeTree — Arena and Basic Operations

**Files:**
- Create: `crates/velox-scene/src/node.rs`
- Create: `crates/velox-scene/src/tree.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests for NodeTree**

Add tests at the bottom of `src/tree.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_root_node() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        assert_eq!(tree.root(), Some(root));
        assert_eq!(tree.parent(root), None);
        assert!(tree.children(root).is_empty());
    }

    #[test]
    fn insert_child_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child1 = tree.insert(Some(root));
        let child2 = tree.insert(Some(root));
        assert_eq!(tree.children(root), &[child1, child2]);
        assert_eq!(tree.parent(child1), Some(root));
        assert_eq!(tree.parent(child2), Some(root));
    }

    #[test]
    fn remove_node_and_descendants() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));
        let grandchild = tree.insert(Some(child));
        tree.remove(child);
        assert!(tree.children(root).is_empty());
        assert!(!tree.contains(child));
        assert!(!tree.contains(grandchild));
    }

    #[test]
    fn reparent_node() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let a = tree.insert(Some(root));
        let b = tree.insert(Some(root));
        let child = tree.insert(Some(a));
        tree.reparent(child, b);
        assert!(tree.children(a).is_empty());
        assert_eq!(tree.children(b), &[child]);
        assert_eq!(tree.parent(child), Some(b));
    }

    #[test]
    fn node_count() {
        let mut tree = NodeTree::new();
        assert_eq!(tree.len(), 0);
        let root = tree.insert(None);
        assert_eq!(tree.len(), 1);
        let _child = tree.insert(Some(root));
        assert_eq!(tree.len(), 2);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — `NodeTree` not defined

**Step 3: Implement NodeId in `src/node.rs`**

```rust
use slotmap::new_key_type;

new_key_type! {
    pub struct NodeId;
}
```

**Step 4: Implement NodeTree in `src/tree.rs`**

```rust
use slotmap::SlotMap;

use crate::geometry::Rect;
use crate::node::NodeId;

pub(crate) struct NodeData {
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) rect: Rect,
    pub(crate) visible: bool,
    pub(crate) layout_dirty: bool,
    pub(crate) paint_dirty: bool,
    pub(crate) hit_test_transparent: bool,
}

impl NodeData {
    fn new(parent: Option<NodeId>) -> Self {
        Self {
            parent,
            children: Vec::new(),
            rect: Rect::zero(),
            visible: true,
            layout_dirty: true,
            paint_dirty: true,
            hit_test_transparent: false,
        }
    }
}

pub struct NodeTree {
    nodes: SlotMap<NodeId, NodeData>,
    root: Option<NodeId>,
}

impl NodeTree {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            root: None,
        }
    }

    pub fn insert(&mut self, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.insert(NodeData::new(parent));
        if let Some(parent_id) = parent {
            if let Some(parent_data) = self.nodes.get_mut(parent_id) {
                parent_data.children.push(id);
                parent_data.layout_dirty = true;
            }
        } else if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    pub fn remove(&mut self, id: NodeId) {
        let descendants = self.collect_descendants(id);
        if let Some(data) = self.nodes.get(id) {
            if let Some(parent_id) = data.parent {
                if let Some(parent_data) = self.nodes.get_mut(parent_id) {
                    parent_data.children.retain(|c| *c != id);
                    parent_data.layout_dirty = true;
                }
            }
        }
        if self.root == Some(id) {
            self.root = None;
        }
        for desc in descendants {
            self.nodes.remove(desc);
        }
        self.nodes.remove(id);
    }

    fn collect_descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            if current != id {
                result.push(current);
            }
            if let Some(data) = self.nodes.get(current) {
                for child in data.children.iter().rev() {
                    stack.push(*child);
                }
            }
        }
        result
    }

    pub fn reparent(&mut self, id: NodeId, new_parent: NodeId) {
        if let Some(data) = self.nodes.get(id) {
            let old_parent = data.parent;
            if let Some(old_parent_id) = old_parent {
                if let Some(old_parent_data) = self.nodes.get_mut(old_parent_id) {
                    old_parent_data.children.retain(|c| *c != id);
                    old_parent_data.layout_dirty = true;
                }
            }
        }
        if let Some(data) = self.nodes.get_mut(id) {
            data.parent = Some(new_parent);
        }
        if let Some(parent_data) = self.nodes.get_mut(new_parent) {
            parent_data.children.push(id);
            parent_data.layout_dirty = true;
        }
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id).and_then(|d| d.parent)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes
            .get(id)
            .map(|d| d.children.as_slice())
            .unwrap_or(&[])
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id)
    }

    pub(crate) fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeData> {
        self.nodes.get_mut(id)
    }
}

impl Default for NodeTree {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 5: Update `src/lib.rs`**

```rust
mod geometry;
mod node;
mod tree;

pub use geometry::{Point, Rect, Size};
pub use node::NodeId;
pub use tree::NodeTree;
```

**Step 6: Run tests**

Run: `cargo test -p velox-scene`
Expected: 5 tests pass

**Step 7: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add NodeTree with insert, remove, reparent, traversal"
```

---

## Task 3: Node Geometry and Dirty Flags

**Files:**
- Modify: `crates/velox-scene/src/tree.rs` (add public geometry/visibility/dirty APIs)

**Step 1: Write failing tests**

Add to the `tests` module in `src/tree.rs`:

```rust
    #[test]
    fn set_rect_marks_paint_dirty() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.clear_dirty(root);
        tree.set_rect(root, Rect::new(10.0, 20.0, 100.0, 50.0));
        let data = tree.get(root).unwrap();
        assert_eq!(data.rect, Rect::new(10.0, 20.0, 100.0, 50.0));
        assert!(data.paint_dirty);
    }

    #[test]
    fn set_visible_marks_paint_dirty() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.clear_dirty(root);
        tree.set_visible(root, false);
        let data = tree.get(root).unwrap();
        assert!(!data.visible);
        assert!(data.paint_dirty);
    }

    #[test]
    fn mark_layout_dirty_propagates_up() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));
        let grandchild = tree.insert(Some(child));
        tree.clear_dirty(root);
        tree.clear_dirty(child);
        tree.clear_dirty(grandchild);
        tree.mark_layout_dirty(grandchild);
        assert!(tree.get(grandchild).unwrap().layout_dirty);
        assert!(tree.get(child).unwrap().layout_dirty);
        assert!(tree.get(root).unwrap().layout_dirty);
    }

    #[test]
    fn mark_paint_dirty() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.clear_dirty(root);
        tree.mark_paint_dirty(root);
        assert!(tree.get(root).unwrap().paint_dirty);
    }

    #[test]
    fn rect_and_visible_getters() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(5.0, 10.0, 200.0, 100.0));
        assert_eq!(tree.rect(root), Some(Rect::new(5.0, 10.0, 200.0, 100.0)));
        assert_eq!(tree.is_visible(root), Some(true));
        tree.set_visible(root, false);
        assert_eq!(tree.is_visible(root), Some(false));
    }

    #[test]
    fn set_hit_test_transparent() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        assert_eq!(tree.is_hit_test_transparent(root), Some(false));
        tree.set_hit_test_transparent(root, true);
        assert_eq!(tree.is_hit_test_transparent(root), Some(true));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — methods not defined

**Step 3: Add public API methods to NodeTree**

Add these methods to the `impl NodeTree` block in `src/tree.rs`:

```rust
    pub fn set_rect(&mut self, id: NodeId, rect: Rect) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.rect = rect;
            data.paint_dirty = true;
        }
    }

    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.nodes.get(id).map(|d| d.rect)
    }

    pub fn set_visible(&mut self, id: NodeId, visible: bool) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.visible = visible;
            data.paint_dirty = true;
        }
    }

    pub fn is_visible(&self, id: NodeId) -> Option<bool> {
        self.nodes.get(id).map(|d| d.visible)
    }

    pub fn set_hit_test_transparent(&mut self, id: NodeId, transparent: bool) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.hit_test_transparent = transparent;
        }
    }

    pub fn is_hit_test_transparent(&self, id: NodeId) -> Option<bool> {
        self.nodes.get(id).map(|d| d.hit_test_transparent)
    }

    pub fn mark_layout_dirty(&mut self, id: NodeId) {
        let mut current = Some(id);
        while let Some(node_id) = current {
            if let Some(data) = self.nodes.get_mut(node_id) {
                if data.layout_dirty {
                    break;
                }
                data.layout_dirty = true;
                current = data.parent;
            } else {
                break;
            }
        }
    }

    pub fn mark_paint_dirty(&mut self, id: NodeId) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.paint_dirty = true;
        }
    }

    pub fn clear_dirty(&mut self, id: NodeId) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.layout_dirty = false;
            data.paint_dirty = false;
        }
    }
```

**Step 4: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add node geometry, visibility, and dirty flag APIs"
```

---

## Task 4: Paint Commands and CommandList

**Files:**
- Create: `crates/velox-scene/src/paint.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests at the bottom of `src/paint.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    #[test]
    fn empty_command_list() {
        let list = CommandList::new();
        assert!(list.commands().is_empty());
    }

    #[test]
    fn push_fill_rect() {
        let mut list = CommandList::new();
        list.fill_rect(Rect::new(0.0, 0.0, 100.0, 50.0), Color::rgba(255, 0, 0, 255));
        assert_eq!(list.commands().len(), 1);
        match &list.commands()[0] {
            PaintCommand::FillRect { rect, color } => {
                assert_eq!(rect.width, 100.0);
                assert_eq!(color.r, 255);
            }
            _ => panic!("expected FillRect"),
        }
    }

    #[test]
    fn push_clip_and_pop() {
        let mut list = CommandList::new();
        list.push_clip(Rect::new(0.0, 0.0, 50.0, 50.0));
        list.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::rgba(0, 0, 0, 255));
        list.pop_clip();
        assert_eq!(list.commands().len(), 3);
    }

    #[test]
    fn clear_resets_list() {
        let mut list = CommandList::new();
        list.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::rgba(0, 0, 0, 255));
        list.clear();
        assert!(list.commands().is_empty());
    }

    #[test]
    fn stroke_rect() {
        let mut list = CommandList::new();
        list.stroke_rect(Rect::new(0.0, 0.0, 50.0, 50.0), Color::rgba(0, 255, 0, 255), 2.0);
        assert_eq!(list.commands().len(), 1);
        match &list.commands()[0] {
            PaintCommand::StrokeRect { width, .. } => assert_eq!(*width, 2.0),
            _ => panic!("expected StrokeRect"),
        }
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — types not defined

**Step 3: Implement paint types in `src/paint.rs`**

```rust
use crate::geometry::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Debug, Clone)]
pub enum PaintCommand {
    FillRect { rect: Rect, color: Color },
    StrokeRect { rect: Rect, color: Color, width: f32 },
    PushClip(Rect),
    PopClip,
}

pub struct CommandList {
    commands: Vec<PaintCommand>,
}

impl CommandList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(PaintCommand::FillRect { rect, color });
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.commands.push(PaintCommand::StrokeRect { rect, color, width });
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.commands.push(PaintCommand::PushClip(rect));
    }

    pub fn pop_clip(&mut self) {
        self.commands.push(PaintCommand::PopClip);
    }

    pub fn commands(&self) -> &[PaintCommand] {
        &self.commands
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for CommandList {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Update `src/lib.rs`**

```rust
mod geometry;
mod node;
mod paint;
mod tree;

pub use geometry::{Point, Rect, Size};
pub use node::NodeId;
pub use paint::{Color, CommandList, PaintCommand};
pub use tree::NodeTree;
```

**Step 5: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add PaintCommand, Color, and CommandList"
```

---

## Task 5: Painter Trait and Paint Pass

**Files:**
- Create: `crates/velox-scene/src/painter.rs`
- Modify: `crates/velox-scene/src/tree.rs` (add painter storage + paint pass)
- Modify: `crates/velox-scene/src/node.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests at the bottom of `src/painter.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::paint::{Color, CommandList, PaintCommand};
    use crate::tree::NodeTree;

    struct FillPainter {
        color: Color,
    }

    impl Painter for FillPainter {
        fn paint(&self, rect: Rect, commands: &mut CommandList) {
            commands.fill_rect(rect, self.color);
        }
    }

    #[test]
    fn paint_pass_collects_commands() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_painter(root, FillPainter { color: Color::rgb(255, 0, 0) });

        let child = tree.insert(Some(root));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 30.0));
        tree.set_painter(child, FillPainter { color: Color::rgb(0, 255, 0) });

        let mut commands = CommandList::new();
        tree.run_paint(&mut commands);

        let cmds = commands.commands();
        assert!(cmds.len() >= 4);
        assert!(matches!(cmds[0], PaintCommand::PushClip(_)));
        assert!(matches!(cmds[1], PaintCommand::FillRect { .. }));
    }

    #[test]
    fn paint_skips_invisible_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_painter(root, FillPainter { color: Color::rgb(255, 0, 0) });
        tree.set_visible(root, false);

        let mut commands = CommandList::new();
        tree.run_paint(&mut commands);
        assert!(commands.commands().is_empty());
    }

    #[test]
    fn paint_clears_dirty_flags() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_painter(root, FillPainter { color: Color::rgb(0, 0, 255) });

        assert!(tree.get(root).unwrap().paint_dirty);
        let mut commands = CommandList::new();
        tree.run_paint(&mut commands);
        assert!(!tree.get(root).unwrap().paint_dirty);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — Painter trait and set_painter not defined

**Step 3: Create Painter trait in `src/painter.rs`**

```rust
use crate::geometry::Rect;
use crate::paint::CommandList;

pub trait Painter {
    fn paint(&self, rect: Rect, commands: &mut CommandList);
}
```

**Step 4: Add painter storage to NodeData and paint pass to NodeTree**

In `src/tree.rs`, update `NodeData` to add:

```rust
use crate::painter::Painter;
use crate::paint::CommandList;
```

Add to `NodeData`:
```rust
pub(crate) painter: Option<Box<dyn Painter>>,
```

Initialize it as `None` in `NodeData::new()`.

Add to `impl NodeTree`:

```rust
    pub fn set_painter(&mut self, id: NodeId, painter: impl Painter + 'static) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.painter = Some(Box::new(painter));
            data.paint_dirty = true;
        }
    }

    pub fn run_paint(&mut self, commands: &mut CommandList) {
        if let Some(root) = self.root {
            self.paint_node(root, commands);
        }
    }

    fn paint_node(&mut self, id: NodeId, commands: &mut CommandList) {
        let (rect, visible, children, has_painter) = {
            let Some(data) = self.nodes.get(id) else { return };
            if !data.visible {
                return;
            }
            (data.rect, data.visible, data.children.clone(), data.painter.is_some())
        };

        commands.push_clip(rect);

        if has_painter {
            let data = self.nodes.get(id).unwrap();
            let painter = data.painter.as_ref().unwrap();
            painter.paint(rect, commands);
        }

        for child in children {
            self.paint_node(child, commands);
        }

        commands.pop_clip();

        if let Some(data) = self.nodes.get_mut(id) {
            data.paint_dirty = false;
        }
    }
```

Note: The `paint_node` method needs to borrow the painter immutably while passing `commands` mutably. Since `Painter` is behind a `Box<dyn Painter>` stored in the node, we need to temporarily extract it. Here's the corrected approach — extract the painter, call it, put it back:

```rust
    fn paint_node(&mut self, id: NodeId, commands: &mut CommandList) {
        let Some(data) = self.nodes.get(id) else { return };
        if !data.visible {
            return;
        }
        let rect = data.rect;
        let children = data.children.clone();

        commands.push_clip(rect);

        let painter = self.nodes.get_mut(id).and_then(|d| d.painter.take());
        if let Some(ref p) = painter {
            p.paint(rect, commands);
        }
        if let Some(data) = self.nodes.get_mut(id) {
            data.painter = painter;
        }

        for child in children {
            self.paint_node(child, commands);
        }

        commands.pop_clip();

        if let Some(data) = self.nodes.get_mut(id) {
            data.paint_dirty = false;
        }
    }
```

**Step 5: Update `src/lib.rs`**

```rust
mod geometry;
mod node;
mod paint;
mod painter;
mod tree;

pub use geometry::{Point, Rect, Size};
pub use node::NodeId;
pub use paint::{Color, CommandList, PaintCommand};
pub use painter::Painter;
pub use tree::NodeTree;
```

**Step 6: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 7: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add Painter trait and paint pass with clip stack"
```

---

## Task 6: Layout Trait and Built-in Helpers

**Files:**
- Create: `crates/velox-scene/src/layout.rs`
- Modify: `crates/velox-scene/src/tree.rs` (add layout storage + layout pass)
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests at the bottom of `src/layout.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::tree::NodeTree;

    #[test]
    fn stack_layout_vertical() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 300.0));
        tree.set_layout(root, StackLayout { direction: Direction::Vertical, spacing: 10.0 });

        let c1 = tree.insert(Some(root));
        let c2 = tree.insert(Some(root));
        let c3 = tree.insert(Some(root));

        tree.run_layout();

        let r1 = tree.rect(c1).unwrap();
        let r2 = tree.rect(c2).unwrap();
        let r3 = tree.rect(c3).unwrap();

        assert_eq!(r1.x, 0.0);
        assert_eq!(r1.y, 0.0);
        assert_eq!(r1.width, 200.0);

        assert_eq!(r2.x, 0.0);
        assert!(r2.y > r1.y);

        assert_eq!(r3.x, 0.0);
        assert!(r3.y > r2.y);
    }

    #[test]
    fn stack_layout_horizontal() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 300.0, 100.0));
        tree.set_layout(root, StackLayout { direction: Direction::Horizontal, spacing: 5.0 });

        let c1 = tree.insert(Some(root));
        let c2 = tree.insert(Some(root));

        tree.run_layout();

        let r1 = tree.rect(c1).unwrap();
        let r2 = tree.rect(c2).unwrap();

        assert_eq!(r1.y, 0.0);
        assert_eq!(r1.height, 100.0);
        assert_eq!(r2.y, 0.0);
        assert!(r2.x > r1.x);
    }

    #[test]
    fn padding_layout() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_layout(root, PaddingLayout {
            top: 10.0,
            right: 20.0,
            bottom: 10.0,
            left: 20.0,
        });

        let child = tree.insert(Some(root));
        tree.run_layout();

        let cr = tree.rect(child).unwrap();
        assert_eq!(cr.x, 20.0);
        assert_eq!(cr.y, 10.0);
        assert_eq!(cr.width, 160.0);
        assert_eq!(cr.height, 80.0);
    }

    #[test]
    fn layout_only_visits_dirty_subtrees() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_layout(root, StackLayout { direction: Direction::Vertical, spacing: 0.0 });
        let child = tree.insert(Some(root));

        tree.run_layout();

        let rect_after_first = tree.rect(child).unwrap();
        tree.set_rect(child, Rect::new(99.0, 99.0, 1.0, 1.0));
        assert!(!tree.get(root).unwrap().layout_dirty);
        assert_eq!(tree.rect(child).unwrap(), Rect::new(99.0, 99.0, 1.0, 1.0));
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — Layout types not defined

**Step 3: Implement Layout trait and helpers in `src/layout.rs`**

```rust
use crate::geometry::{Rect, Size};
use crate::node::NodeId;
use crate::tree::NodeTree;

pub trait Layout {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub struct StackLayout {
    pub direction: Direction,
    pub spacing: f32,
}

impl Layout for StackLayout {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree) {
        if children.is_empty() {
            return;
        }
        let count = children.len() as f32;
        match self.direction {
            Direction::Vertical => {
                let total_spacing = self.spacing * (count - 1.0);
                let child_height = (parent_rect.height - total_spacing) / count;
                for (i, &child_id) in children.iter().enumerate() {
                    let y = parent_rect.y + (child_height + self.spacing) * i as f32;
                    tree.set_rect(child_id, Rect::new(parent_rect.x, y, parent_rect.width, child_height));
                }
            }
            Direction::Horizontal => {
                let total_spacing = self.spacing * (count - 1.0);
                let child_width = (parent_rect.width - total_spacing) / count;
                for (i, &child_id) in children.iter().enumerate() {
                    let x = parent_rect.x + (child_width + self.spacing) * i as f32;
                    tree.set_rect(child_id, Rect::new(x, parent_rect.y, child_width, parent_rect.height));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PaddingLayout {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Layout for PaddingLayout {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree) {
        if let Some(&child_id) = children.first() {
            let rect = Rect::new(
                parent_rect.x + self.left,
                parent_rect.y + self.top,
                parent_rect.width - self.left - self.right,
                parent_rect.height - self.top - self.bottom,
            );
            tree.set_rect(child_id, rect);
        }
    }
}
```

**Step 4: Add layout storage and layout pass to NodeTree**

In `src/tree.rs`, add to imports:

```rust
use crate::layout::Layout;
```

Add to `NodeData`:
```rust
pub(crate) layout: Option<Box<dyn Layout>>,
```

Initialize it as `None` in `NodeData::new()`.

Add to `impl NodeTree`:

```rust
    pub fn set_layout(&mut self, id: NodeId, layout: impl Layout + 'static) {
        if let Some(data) = self.nodes.get_mut(id) {
            data.layout = Some(Box::new(layout));
            data.layout_dirty = true;
        }
    }

    pub fn run_layout(&mut self) {
        if let Some(root) = self.root {
            self.layout_node(root);
        }
    }

    fn layout_node(&mut self, id: NodeId) {
        let Some(data) = self.nodes.get(id) else { return };
        let is_dirty = data.layout_dirty;
        let rect = data.rect;
        let children = data.children.clone();
        let has_layout = data.layout.is_some();

        if is_dirty && has_layout {
            let layout = self.nodes.get_mut(id).and_then(|d| d.layout.take());
            if let Some(ref l) = layout {
                l.compute(rect, &children, self);
            }
            if let Some(data) = self.nodes.get_mut(id) {
                data.layout = layout;
            }
        }

        if let Some(data) = self.nodes.get_mut(id) {
            data.layout_dirty = false;
        }

        for child in children {
            self.layout_node(child);
        }
    }
```

**Step 5: Update `src/lib.rs`**

```rust
mod geometry;
mod layout;
mod node;
mod paint;
mod painter;
mod tree;

pub use geometry::{Point, Rect, Size};
pub use layout::{Direction, Layout, PaddingLayout, StackLayout};
pub use node::NodeId;
pub use paint::{Color, CommandList, PaintCommand};
pub use painter::Painter;
pub use tree::NodeTree;
```

**Step 6: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 7: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add Layout trait with StackLayout and PaddingLayout"
```

---

## Task 7: Hit Testing

**Files:**
- Create: `crates/velox-scene/src/hit_test.rs`
- Modify: `crates/velox-scene/src/tree.rs` (add hit_test method)
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests in `src/hit_test.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::geometry::{Point, Rect};
    use crate::tree::NodeTree;

    #[test]
    fn hit_test_returns_deepest_node() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = tree.insert(Some(root));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 50.0));
        let grandchild = tree.insert(Some(child));
        tree.set_rect(grandchild, Rect::new(15.0, 15.0, 20.0, 20.0));

        assert_eq!(tree.hit_test(Point::new(20.0, 20.0)), Some(grandchild));
    }

    #[test]
    fn hit_test_returns_parent_when_miss_child() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = tree.insert(Some(root));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 50.0));

        assert_eq!(tree.hit_test(Point::new(100.0, 100.0)), Some(root));
    }

    #[test]
    fn hit_test_returns_none_outside_root() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));

        assert_eq!(tree.hit_test(Point::new(150.0, 150.0)), None);
    }

    #[test]
    fn hit_test_skips_invisible_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = tree.insert(Some(root));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 50.0));
        tree.set_visible(child, false);

        assert_eq!(tree.hit_test(Point::new(20.0, 20.0)), Some(root));
    }

    #[test]
    fn hit_test_skips_transparent_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        let child = tree.insert(Some(root));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 50.0));
        tree.set_hit_test_transparent(child, true);

        assert_eq!(tree.hit_test(Point::new(20.0, 20.0)), Some(root));
    }

    #[test]
    fn hit_test_last_child_has_priority() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        let c1 = tree.insert(Some(root));
        tree.set_rect(c1, Rect::new(0.0, 0.0, 100.0, 100.0));
        let c2 = tree.insert(Some(root));
        tree.set_rect(c2, Rect::new(0.0, 0.0, 100.0, 100.0));

        assert_eq!(tree.hit_test(Point::new(50.0, 50.0)), Some(c2));
    }

    #[test]
    fn hit_test_empty_tree() {
        let tree = NodeTree::new();
        assert_eq!(tree.hit_test(Point::new(0.0, 0.0)), None);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — `hit_test` not defined

**Step 3: Implement hit testing on NodeTree**

Add to `src/tree.rs`:

```rust
use crate::geometry::Point;
```

Add methods to `impl NodeTree`:

```rust
    pub fn hit_test(&self, point: Point) -> Option<NodeId> {
        self.root.and_then(|root| self.hit_test_node(root, point))
    }

    fn hit_test_node(&self, id: NodeId, point: Point) -> Option<NodeId> {
        let data = self.nodes.get(id)?;

        if !data.visible || data.hit_test_transparent {
            return None;
        }

        if !data.rect.contains(point) {
            return None;
        }

        for &child in data.children.iter().rev() {
            if let Some(hit) = self.hit_test_node(child, point) {
                return Some(hit);
            }
        }

        Some(id)
    }
```

**Step 4: Create `src/hit_test.rs`** (tests only, logic lives on NodeTree)

```rust
// Hit test logic is implemented on NodeTree in tree.rs.
// Tests live here to keep tree.rs focused.

#[cfg(test)]
mod tests {
    // ... tests from Step 1
}
```

**Step 5: Update `src/lib.rs`**

Add `mod hit_test;` to the module list (no public exports — it's just tests).

**Step 6: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 7: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add hit testing with depth-first traversal"
```

---

## Task 8: Focus State

**Files:**
- Create: `crates/velox-scene/src/focus.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests in `src/focus.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeId;
    use slotmap::SlotMap;
    use std::cell::Cell;
    use std::rc::Rc;

    fn make_node_id() -> NodeId {
        let mut sm = SlotMap::<NodeId, ()>::with_key();
        sm.insert(())
    }

    #[test]
    fn initially_no_focus() {
        let focus = FocusState::new();
        assert!(focus.focused().is_none());
    }

    #[test]
    fn request_and_release_focus() {
        let mut focus = FocusState::new();
        let id = make_node_id();
        focus.request_focus(id);
        assert_eq!(focus.focused(), Some(id));
        focus.release_focus();
        assert!(focus.focused().is_none());
    }

    #[test]
    fn focus_change_emits_event() {
        let mut focus = FocusState::new();
        let id1 = make_node_id();
        let id2 = make_node_id();

        let changes: Rc<Cell<u32>> = Rc::new(Cell::new(0));
        let changes_clone = changes.clone();
        let _sub = focus.on_focus_change(move |_change| {
            changes_clone.set(changes_clone.get() + 1);
        });

        focus.request_focus(id1);
        assert_eq!(changes.get(), 1);

        focus.request_focus(id2);
        assert_eq!(changes.get(), 2);

        focus.release_focus();
        assert_eq!(changes.get(), 3);
    }

    #[test]
    fn focus_change_contains_lost_and_gained() {
        let mut focus = FocusState::new();
        let id1 = make_node_id();
        let id2 = make_node_id();

        focus.request_focus(id1);

        let last_change: Rc<Cell<Option<FocusChange>>> = Rc::new(Cell::new(None));
        let lc = last_change.clone();
        let _sub = focus.on_focus_change(move |change| {
            lc.set(Some(change.clone()));
        });

        focus.request_focus(id2);
        let change = last_change.get().unwrap();
        assert_eq!(change.lost, Some(id1));
        assert_eq!(change.gained, Some(id2));
    }

    #[test]
    fn request_same_focus_is_noop() {
        let mut focus = FocusState::new();
        let id = make_node_id();
        focus.request_focus(id);

        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let _sub = focus.on_focus_change(move |_| {
            c.set(c.get() + 1);
        });

        focus.request_focus(id);
        assert_eq!(count.get(), 0);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — FocusState not defined

**Step 3: Implement FocusState in `src/focus.rs`**

```rust
use velox_reactive::{Event, Subscription};

use crate::node::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusChange {
    pub lost: Option<NodeId>,
    pub gained: Option<NodeId>,
}

pub struct FocusState {
    focused: Option<NodeId>,
    change_event: Event<FocusChange>,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            focused: None,
            change_event: Event::new(),
        }
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    pub fn request_focus(&mut self, id: NodeId) {
        if self.focused == Some(id) {
            return;
        }
        let lost = self.focused;
        self.focused = Some(id);
        self.change_event.emit(FocusChange {
            lost,
            gained: Some(id),
        });
    }

    pub fn release_focus(&mut self) {
        if self.focused.is_none() {
            return;
        }
        let lost = self.focused;
        self.focused = None;
        self.change_event.emit(FocusChange {
            lost,
            gained: None,
        });
    }

    pub fn on_focus_change(&self, callback: impl Fn(&FocusChange) + 'static) -> Subscription {
        self.change_event.subscribe(callback)
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Update `src/lib.rs`**

Add `mod focus;` and export:

```rust
pub use focus::{FocusChange, FocusState};
```

**Step 5: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add FocusState with change events"
```

---

## Task 9: Overlay Stack

**Files:**
- Create: `crates/velox-scene/src/overlay.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests in `src/overlay.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rect};

    #[test]
    fn push_and_pop_overlay() {
        let mut stack = OverlayStack::new();
        assert!(stack.is_empty());
        let id = stack.push_overlay();
        assert!(!stack.is_empty());
        assert_eq!(stack.len(), 1);
        stack.pop_overlay(id);
        assert!(stack.is_empty());
    }

    #[test]
    fn access_overlay_tree() {
        let mut stack = OverlayStack::new();
        let id = stack.push_overlay();
        let tree = stack.overlay_tree_mut(id).unwrap();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(tree.rect(root), Some(Rect::new(0.0, 0.0, 100.0, 50.0)));
    }

    #[test]
    fn dismiss_all() {
        let mut stack = OverlayStack::new();
        stack.push_overlay();
        stack.push_overlay();
        stack.push_overlay();
        assert_eq!(stack.len(), 3);
        stack.dismiss_all();
        assert!(stack.is_empty());
    }

    #[test]
    fn hit_test_checks_topmost_first() {
        let mut stack = OverlayStack::new();

        let id1 = stack.push_overlay();
        let tree1 = stack.overlay_tree_mut(id1).unwrap();
        let root1 = tree1.insert(None);
        tree1.set_rect(root1, Rect::new(0.0, 0.0, 200.0, 200.0));

        let id2 = stack.push_overlay();
        let tree2 = stack.overlay_tree_mut(id2).unwrap();
        let root2 = tree2.insert(None);
        tree2.set_rect(root2, Rect::new(0.0, 0.0, 100.0, 100.0));

        let (overlay_id, node_id) = stack.hit_test(Point::new(50.0, 50.0)).unwrap();
        assert_eq!(overlay_id, id2);
        assert_eq!(node_id, root2);
    }

    #[test]
    fn hit_test_falls_through_to_lower_overlay() {
        let mut stack = OverlayStack::new();

        let id1 = stack.push_overlay();
        let tree1 = stack.overlay_tree_mut(id1).unwrap();
        let root1 = tree1.insert(None);
        tree1.set_rect(root1, Rect::new(0.0, 0.0, 200.0, 200.0));

        let id2 = stack.push_overlay();
        let tree2 = stack.overlay_tree_mut(id2).unwrap();
        let root2 = tree2.insert(None);
        tree2.set_rect(root2, Rect::new(0.0, 0.0, 50.0, 50.0));

        let (overlay_id, node_id) = stack.hit_test(Point::new(100.0, 100.0)).unwrap();
        assert_eq!(overlay_id, id1);
        assert_eq!(node_id, root1);
    }

    #[test]
    fn hit_test_returns_none_when_empty() {
        let stack = OverlayStack::new();
        assert!(stack.hit_test(Point::new(0.0, 0.0)).is_none());
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — OverlayStack not defined

**Step 3: Implement OverlayStack in `src/overlay.rs`**

```rust
use crate::geometry::Point;
use crate::node::NodeId;
use crate::tree::NodeTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayId(u64);

struct OverlayEntry {
    id: OverlayId,
    tree: NodeTree,
}

pub struct OverlayStack {
    overlays: Vec<OverlayEntry>,
    next_id: u64,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self {
            overlays: Vec::new(),
            next_id: 0,
        }
    }

    pub fn push_overlay(&mut self) -> OverlayId {
        let id = OverlayId(self.next_id);
        self.next_id += 1;
        self.overlays.push(OverlayEntry {
            id,
            tree: NodeTree::new(),
        });
        id
    }

    pub fn pop_overlay(&mut self, id: OverlayId) -> bool {
        let len_before = self.overlays.len();
        self.overlays.retain(|e| e.id != id);
        self.overlays.len() < len_before
    }

    pub fn overlay_tree(&self, id: OverlayId) -> Option<&NodeTree> {
        self.overlays.iter().find(|e| e.id == id).map(|e| &e.tree)
    }

    pub fn overlay_tree_mut(&mut self, id: OverlayId) -> Option<&mut NodeTree> {
        self.overlays.iter_mut().find(|e| e.id == id).map(|e| &mut e.tree)
    }

    pub fn dismiss_all(&mut self) {
        self.overlays.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    pub fn hit_test(&self, point: Point) -> Option<(OverlayId, NodeId)> {
        for entry in self.overlays.iter().rev() {
            if let Some(node_id) = entry.tree.hit_test(point) {
                return Some((entry.id, node_id));
            }
        }
        None
    }
}

impl Default for OverlayStack {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Update `src/lib.rs`**

Add `mod overlay;` and export:

```rust
pub use overlay::{OverlayId, OverlayStack};
```

**Step 5: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add OverlayStack with hit testing priority"
```

---

## Task 10: Scene Struct

**Files:**
- Create: `crates/velox-scene/src/scene.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write failing tests**

Add tests in `src/scene.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rect};
    use crate::layout::{Direction, StackLayout};
    use crate::paint::{Color, PaintCommand};

    struct TestPainter(Color);
    impl crate::painter::Painter for TestPainter {
        fn paint(&self, rect: Rect, commands: &mut crate::paint::CommandList) {
            commands.fill_rect(rect, self.0);
        }
    }

    #[test]
    fn scene_layout_then_paint() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene.tree_mut().set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        scene.tree_mut().set_layout(root, StackLayout {
            direction: Direction::Vertical,
            spacing: 0.0,
        });
        scene.tree_mut().set_painter(root, TestPainter(Color::rgb(255, 0, 0)));

        let child = scene.tree_mut().insert(Some(root));
        scene.tree_mut().set_painter(child, TestPainter(Color::rgb(0, 255, 0)));

        scene.layout();
        scene.paint();

        assert!(!scene.commands().commands().is_empty());
    }

    #[test]
    fn scene_hit_test_checks_overlays_first() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene.tree_mut().set_rect(root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let overlay_id = scene.push_overlay();
        let overlay_tree = scene.overlay_stack_mut().overlay_tree_mut(overlay_id).unwrap();
        let overlay_root = overlay_tree.insert(None);
        overlay_tree.set_rect(overlay_root, Rect::new(0.0, 0.0, 100.0, 100.0));

        let result = scene.hit_test(Point::new(50.0, 50.0));
        assert_eq!(result, Some(overlay_root));
    }

    #[test]
    fn scene_hit_test_falls_through_to_main_tree() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene.tree_mut().set_rect(root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let overlay_id = scene.push_overlay();
        let overlay_tree = scene.overlay_stack_mut().overlay_tree_mut(overlay_id).unwrap();
        let overlay_root = overlay_tree.insert(None);
        overlay_tree.set_rect(overlay_root, Rect::new(0.0, 0.0, 50.0, 50.0));

        let result = scene.hit_test(Point::new(200.0, 200.0));
        assert_eq!(result, Some(root));
    }

    #[test]
    fn scene_focus() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene.focus_mut().request_focus(root);
        assert_eq!(scene.focus().focused(), Some(root));
    }

    #[test]
    fn scene_paint_clears_and_rebuilds_commands() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene.tree_mut().set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));
        scene.tree_mut().set_painter(root, TestPainter(Color::rgb(0, 0, 0)));

        scene.paint();
        let count1 = scene.commands().commands().len();
        assert!(count1 > 0);

        scene.paint();
        let count2 = scene.commands().commands().len();
        assert_eq!(count1, count2);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p velox-scene`
Expected: FAIL — Scene not defined

**Step 3: Implement Scene in `src/scene.rs`**

```rust
use crate::focus::FocusState;
use crate::geometry::Point;
use crate::node::NodeId;
use crate::overlay::{OverlayId, OverlayStack};
use crate::paint::CommandList;
use crate::tree::NodeTree;

pub struct Scene {
    tree: NodeTree,
    overlay_stack: OverlayStack,
    focus: FocusState,
    command_list: CommandList,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            overlay_stack: OverlayStack::new(),
            focus: FocusState::new(),
            command_list: CommandList::new(),
        }
    }

    pub fn tree(&self) -> &NodeTree {
        &self.tree
    }

    pub fn tree_mut(&mut self) -> &mut NodeTree {
        &mut self.tree
    }

    pub fn overlay_stack(&self) -> &OverlayStack {
        &self.overlay_stack
    }

    pub fn overlay_stack_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlay_stack
    }

    pub fn focus(&self) -> &FocusState {
        &self.focus
    }

    pub fn focus_mut(&mut self) -> &mut FocusState {
        &mut self.focus
    }

    pub fn push_overlay(&mut self) -> OverlayId {
        self.overlay_stack.push_overlay()
    }

    pub fn layout(&mut self) {
        self.tree.run_layout();
        for overlay_id in self.overlay_ids() {
            if let Some(tree) = self.overlay_stack.overlay_tree_mut(overlay_id) {
                tree.run_layout();
            }
        }
    }

    pub fn paint(&mut self) {
        self.command_list.clear();
        self.tree.run_paint(&mut self.command_list);
        for overlay_id in self.overlay_ids() {
            if let Some(tree) = self.overlay_stack.overlay_tree_mut(overlay_id) {
                tree.run_paint(&mut self.command_list);
            }
        }
    }

    pub fn hit_test(&self, point: Point) -> Option<NodeId> {
        if let Some((_overlay_id, node_id)) = self.overlay_stack.hit_test(point) {
            return Some(node_id);
        }
        self.tree.hit_test(point)
    }

    pub fn commands(&self) -> &CommandList {
        &self.command_list
    }

    fn overlay_ids(&self) -> Vec<OverlayId> {
        (0..self.overlay_stack.len() as u64)
            .filter_map(|_| None::<OverlayId>)
            .collect()
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
```

**Important:** The `overlay_ids()` helper above is a placeholder. The `OverlayStack` needs an `ids()` method. Add to `src/overlay.rs`:

```rust
    pub fn ids(&self) -> Vec<OverlayId> {
        self.overlays.iter().map(|e| e.id).collect()
    }
```

Then fix `Scene::overlay_ids()`:

```rust
    fn overlay_ids(&self) -> Vec<OverlayId> {
        self.overlay_stack.ids()
    }
```

**Step 4: Update `src/lib.rs`**

```rust
mod focus;
mod geometry;
mod hit_test;
mod layout;
mod node;
mod overlay;
mod paint;
mod painter;
mod scene;
mod tree;

pub use focus::{FocusChange, FocusState};
pub use geometry::{Point, Rect, Size};
pub use layout::{Direction, Layout, PaddingLayout, StackLayout};
pub use node::NodeId;
pub use overlay::{OverlayId, OverlayStack};
pub use paint::{Color, CommandList, PaintCommand};
pub use painter::Painter;
pub use scene::Scene;
pub use tree::NodeTree;
```

**Step 5: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 6: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add Scene struct orchestrating tree, overlays, focus, and paint"
```

---

## Task 11: Integrate Scene into velox-app

**Files:**
- Modify: `crates/velox-app/Cargo.toml` (add velox-scene dependency)
- Modify: `crates/velox-app/src/handler.rs`
- Modify: `crates/velox/Cargo.toml` (add velox-scene dependency)
- Modify: `crates/velox/src/lib.rs` (re-export scene)

**Step 1: Add velox-scene dependency to velox-app**

In `crates/velox-app/Cargo.toml`, add:

```toml
velox-scene = { workspace = true }
```

**Step 2: Update VeloxHandler to own scenes**

In `crates/velox-app/src/handler.rs`:

```rust
use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_runtime::Runtime;
use velox_scene::Scene;
use velox_window::{WindowConfig, WindowId, WindowManager};

pub(crate) struct VeloxHandler {
    runtime: Runtime,
    window_manager: WindowManager,
    scenes: HashMap<WindowId, Scene>,
    pending_windows: Vec<WindowConfig>,
    initialized: bool,
}

impl VeloxHandler {
    pub(crate) fn new(runtime: Runtime, window_configs: Vec<WindowConfig>) -> Self {
        Self {
            runtime,
            window_manager: WindowManager::new(),
            scenes: HashMap::new(),
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

        let configs: Vec<WindowConfig> = self.pending_windows.drain(..).collect();
        if configs.is_empty() {
            event_loop.exit();
            return;
        }

        for config in configs {
            match self.window_manager.create_window(event_loop, config) {
                Ok(window_id) => {
                    self.scenes.insert(window_id, Scene::new());
                }
                Err(_) => {}
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let velox_id = WindowId::from_winit(window_id);
        match event {
            WindowEvent::CloseRequested => {
                self.window_manager.close_by_winit_id(window_id);
                self.scenes.remove(&velox_id);
                if self.window_manager.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(_) => {}
            WindowEvent::RedrawRequested => {
                if let Some(scene) = self.scenes.get_mut(&velox_id) {
                    scene.layout();
                    scene.paint();
                }
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

**Note:** `WindowId::from_winit` is currently `pub(crate)` in velox-window. It needs to be `pub` for velox-app to use it. Update `crates/velox-window/src/window_id.rs`:

Change `pub(crate) fn from_winit` to `pub fn from_winit`.

**Step 3: Update velox facade**

In `crates/velox/Cargo.toml`, add:

```toml
velox-scene = { workspace = true }
```

In `crates/velox/src/lib.rs`:

```rust
pub use velox_app as app;
pub use velox_platform as platform;
pub use velox_reactive as reactive;
pub use velox_runtime as runtime;
pub use velox_scene as scene;
pub use velox_window as window;

pub mod prelude {
    pub use velox_app::App;
    pub use velox_reactive::{Batch, Computed, Event, Signal, Subscription, SubscriptionBag};
    pub use velox_runtime::{PowerClass, PowerPolicy};
    pub use velox_scene::{Scene, NodeId, NodeTree, Rect, Point, Size};
    pub use velox_window::WindowConfig;
}
```

**Step 4: Verify full workspace builds**

Run: `cargo build --workspace`
Expected: compiles with no errors

**Step 5: Run all tests**

Run: `cargo test --workspace`
Expected: all tests pass across all crates

**Step 6: Commit**

```bash
git add crates/velox-app/ crates/velox-window/ crates/velox/ Cargo.lock
git commit -m "feat(app): integrate Scene into VeloxHandler event loop"
```

---

## Task 12: Update CLAUDE.md and Final Verification

**Files:**
- Modify: `CLAUDE.md`

**Step 1: Run full verification**

Run: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace`
Expected: builds clean, all tests pass, no clippy warnings

**Step 2: Update CLAUDE.md**

Update the status and crate architecture section to reflect Phase 2 completion. Add `velox-scene` to the implemented crates list.

**Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for Phase 2 completion"
```
