# Phase 4: Text Editing, Event Routing, and Commands — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add event routing from winit to focused nodes, an editable text model with cursor/selection/undo, clipboard integration, DrawGlyphs rendering, and a working text input widget.

**Architecture:** Four areas of work: (1) event types and dispatch in velox-scene, (2) editable text model in velox-text with cursor/selection/undo, (3) DrawGlyphs paint command through velox-render, (4) wiring in velox-app with clipboard via arboard. A TextInputHandler implements EventHandler to tie it all together.

**Tech Stack:** Rust (edition 2024), cosmic-text 0.12, wgpu 24, arboard (clipboard)

---

## Context

### Existing Crates (Phases 1-3)

- `velox-reactive` — Signal, Computed, Event, Batch, Subscription
- `velox-runtime` — Runtime, FrameClock, PowerPolicy, task executors
- `velox-platform` — platform traits with stub impls
- `velox-window` — WindowConfig, WindowId, WindowManager (ManagedWindow stores Arc<Window>)
- `velox-scene` — NodeTree, Scene, Painter, Layout, CommandList, PaintCommand, FocusState, OverlayStack, ShortcutRegistry (Key, Modifiers, KeyCombo)
- `velox-text` — FontSystem, TextBuffer, TextAttrs, GlyphRasterizer, RasterizedGlyph
- `velox-render` — GpuContext, WindowSurface, RectRenderer, GlyphAtlas, GlyphRenderer, Renderer
- `velox-app` — App builder (with setup callback), VeloxHandler (ApplicationHandler impl)
- `velox` — facade re-exports + prelude

### Key Files

- `crates/velox-scene/src/shortcut.rs` — Key enum (A-Z, Num0-9, F1-12, arrows, etc.), Modifiers bitflags (SHIFT/CTRL/ALT/SUPER), KeyCombo, ShortcutRegistry
- `crates/velox-scene/src/focus.rs` — FocusState with focused(), request_focus(), release_focus(), on_focus_change()
- `crates/velox-scene/src/tree.rs` — NodeTree with SlotMap<NodeId, NodeData>. NodeData has painter/layout as Option<Box<dyn Trait>>. Uses take/put-back pattern for calling trait methods.
- `crates/velox-scene/src/paint.rs` — PaintCommand enum (FillRect, StrokeRect, PushClip, PopClip), Color (r,g,b,a: u8), CommandList
- `crates/velox-text/src/buffer.rs` — TextBuffer wrapping cosmic_text::Buffer
- `crates/velox-app/src/handler.rs` — VeloxHandler: handles CloseRequested, Resized, RedrawRequested. Does NOT handle KeyboardInput or mouse events yet.
- `crates/velox-render/src/renderer.rs` — Renderer::render() matches on PaintCommand variants, dispatches to RectRenderer

### Patterns

- **Take/put-back:** For calling trait objects on nodes: `let painter = node.painter.take(); painter.paint(...); node.painter = painter;`
- **Trait on NodeData:** `set_painter(id, impl Painter)` stores `Box<dyn Painter>` in NodeData
- **Event dispatch:** ShortcutRegistry.handle_key_event() returns bool if consumed

---

## Task 1: Event Types (KeyEvent, MouseEvent)

**Files:**
- Create: `crates/velox-scene/src/event.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut::{Key, Modifiers};

    #[test]
    fn key_event_with_text() {
        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: KeyState::Pressed,
            text: Some("a".into()),
        };
        assert_eq!(event.text.as_deref(), Some("a"));
        assert!(event.state.is_pressed());
    }

    #[test]
    fn key_event_modifier_only() {
        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::CTRL,
            state: KeyState::Pressed,
            text: None,
        };
        assert!(event.modifiers.contains(Modifiers::CTRL));
        assert!(event.text.is_none());
    }

    #[test]
    fn mouse_event_click() {
        let event = MouseEvent {
            position: crate::Point::new(50.0, 30.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 1,
            modifiers: Modifiers::empty(),
        };
        assert_eq!(event.click_count, 1);
        assert!(event.state.is_pressed());
    }

    #[test]
    fn mouse_event_double_click() {
        let event = MouseEvent {
            position: crate::Point::new(50.0, 30.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 2,
            modifiers: Modifiers::empty(),
        };
        assert_eq!(event.click_count, 2);
    }
}
```

**Step 2: Implement event.rs**

```rust
use crate::geometry::Point;
use crate::shortcut::{Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

impl KeyState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub state: KeyState,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub position: Point,
    pub button: MouseButton,
    pub state: ButtonState,
    pub click_count: u32,
    pub modifiers: Modifiers,
}
```

**Step 3: Update lib.rs**

Add `mod event;` and:
```rust
pub use event::{ButtonState, KeyEvent, KeyState, MouseButton, MouseEvent};
```

**Step 4: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add KeyEvent, MouseEvent, KeyState, ButtonState types"
```

---

## Task 2: EventHandler Trait and EventContext

**Files:**
- Create: `crates/velox-scene/src/event_handler.rs`
- Modify: `crates/velox-scene/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ButtonState, KeyEvent, KeyState, MouseButton, MouseEvent};
    use crate::geometry::{Point, Rect};
    use crate::shortcut::{Key, Modifiers};
    use std::cell::Cell;
    use std::rc::Rc;

    struct TestKeyHandler {
        handled: Rc<Cell<bool>>,
    }

    impl EventHandler for TestKeyHandler {
        fn handle_key(&mut self, _event: &KeyEvent, _ctx: &mut EventContext) -> bool {
            self.handled.set(true);
            true
        }
    }

    #[test]
    fn event_handler_receives_key() {
        let handled = Rc::new(Cell::new(false));
        let mut handler = TestKeyHandler {
            handled: handled.clone(),
        };
        let mut ctx = EventContext::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: KeyState::Pressed,
            text: Some("a".into()),
        };
        let consumed = handler.handle_key(&event, &mut ctx);
        assert!(consumed);
        assert!(handled.get());
    }

    #[test]
    fn event_context_request_redraw() {
        let mut ctx = EventContext::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert!(!ctx.redraw_requested());
        ctx.request_redraw();
        assert!(ctx.redraw_requested());
    }

    #[test]
    fn default_mouse_handler_returns_false() {
        struct EmptyHandler;
        impl EventHandler for EmptyHandler {
            fn handle_key(&mut self, _: &KeyEvent, _: &mut EventContext) -> bool {
                false
            }
        }
        let mut handler = EmptyHandler;
        let mut ctx = EventContext::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        let event = MouseEvent {
            position: Point::new(10.0, 10.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 1,
            modifiers: Modifiers::empty(),
        };
        assert!(!handler.handle_mouse(&event, &mut ctx));
    }
}
```

**Step 2: Implement event_handler.rs**

```rust
use crate::event::{KeyEvent, MouseEvent};
use crate::geometry::Rect;

pub struct EventContext {
    rect: Rect,
    redraw: bool,
    clipboard_read: Option<String>,
    clipboard_write: Option<String>,
}

impl EventContext {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            redraw: false,
            clipboard_read: None,
            clipboard_write: None,
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }

    pub fn redraw_requested(&self) -> bool {
        self.redraw
    }

    pub fn set_clipboard_content(&mut self, content: Option<String>) {
        self.clipboard_read = content;
    }

    pub fn clipboard_get(&self) -> Option<&str> {
        self.clipboard_read.as_deref()
    }

    pub fn clipboard_set(&mut self, text: &str) {
        self.clipboard_write = Some(text.to_owned());
    }

    pub fn take_clipboard_write(&mut self) -> Option<String> {
        self.clipboard_write.take()
    }
}

pub trait EventHandler: 'static {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool;

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        let _ = (event, ctx);
        false
    }

    fn handle_focus(&mut self, gained: bool) {
        let _ = gained;
    }
}
```

**Step 3: Update lib.rs**

Add `mod event_handler;` and:
```rust
pub use event_handler::{EventContext, EventHandler};
```

**Step 4: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add EventHandler trait and EventContext"
```

---

## Task 3: Add EventHandler to NodeTree

**Files:**
- Modify: `crates/velox-scene/src/tree.rs`

**Step 1: Write tests (add to tree.rs existing test module)**

```rust
#[test]
fn set_and_dispatch_event_handler() {
    use crate::event::{KeyEvent, KeyState};
    use crate::event_handler::{EventContext, EventHandler};
    use crate::shortcut::{Key, Modifiers};
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountHandler {
        count: Rc<Cell<u32>>,
    }
    impl EventHandler for CountHandler {
        fn handle_key(&mut self, _: &KeyEvent, _: &mut EventContext) -> bool {
            self.count.set(self.count.get() + 1);
            true
        }
    }

    let mut tree = NodeTree::new();
    let root = tree.insert(None);
    tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
    let count = Rc::new(Cell::new(0u32));
    tree.set_event_handler(root, CountHandler { count: count.clone() });

    let event = KeyEvent {
        key: Key::A,
        modifiers: Modifiers::empty(),
        state: KeyState::Pressed,
        text: Some("a".into()),
    };
    let consumed = tree.dispatch_key_event(root, &event);
    assert!(consumed);
    assert_eq!(count.get(), 1);
}

#[test]
fn dispatch_to_node_without_handler_returns_false() {
    use crate::event::{KeyEvent, KeyState};
    use crate::shortcut::{Key, Modifiers};

    let mut tree = NodeTree::new();
    let root = tree.insert(None);
    tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));

    let event = KeyEvent {
        key: Key::A,
        modifiers: Modifiers::empty(),
        state: KeyState::Pressed,
        text: Some("a".into()),
    };
    let consumed = tree.dispatch_key_event(root, &event);
    assert!(!consumed);
}
```

**Step 2: Modify NodeData and NodeTree**

In `tree.rs`, add to NodeData:
```rust
pub(crate) event_handler: Option<Box<dyn EventHandler>>,
```

Initialize to `None` in `NodeData::new()`.

Add import: `use crate::event_handler::{EventContext, EventHandler};`
Add import: `use crate::event::KeyEvent;`

Add methods to NodeTree:

```rust
pub fn set_event_handler(&mut self, id: NodeId, handler: impl EventHandler + 'static) {
    if let Some(node) = self.nodes.get_mut(id) {
        node.event_handler = Some(Box::new(handler));
    }
}

pub fn dispatch_key_event(&mut self, id: NodeId, event: &KeyEvent) -> bool {
    let Some(data) = self.nodes.get(id) else {
        return false;
    };
    let rect = data.rect;
    let handler = self.nodes.get_mut(id).and_then(|d| d.event_handler.take());
    let consumed = if let Some(mut h) = handler {
        let mut ctx = EventContext::new(rect);
        let result = h.handle_key(event, &mut ctx);
        if let Some(data) = self.nodes.get_mut(id) {
            data.event_handler = Some(h);
        }
        result
    } else {
        false
    };
    consumed
}

pub fn dispatch_mouse_event(&mut self, id: NodeId, event: &crate::event::MouseEvent) -> bool {
    let Some(data) = self.nodes.get(id) else {
        return false;
    };
    let rect = data.rect;
    let handler = self.nodes.get_mut(id).and_then(|d| d.event_handler.take());
    let consumed = if let Some(mut h) = handler {
        let mut ctx = EventContext::new(rect);
        let result = h.handle_mouse(event, &mut ctx);
        if let Some(data) = self.nodes.get_mut(id) {
            data.event_handler = Some(h);
        }
        result
    } else {
        false
    };
    consumed
}
```

**Step 3: Run tests**

Run: `cargo test -p velox-scene`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/velox-scene/
git commit -m "feat(scene): add EventHandler storage and dispatch on NodeTree"
```

---

## Task 4: TextSelection and TextPosition

**Files:**
- Create: `crates/velox-text/src/selection.rs`
- Modify: `crates/velox-text/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_selection() {
        let sel = TextSelection::collapsed(5);
        assert!(sel.is_collapsed());
        assert_eq!(sel.range(), (5, 5));
    }

    #[test]
    fn forward_selection() {
        let sel = TextSelection { anchor: 2, focus: 8 };
        assert!(!sel.is_collapsed());
        assert_eq!(sel.range(), (2, 8));
    }

    #[test]
    fn backward_selection() {
        let sel = TextSelection { anchor: 10, focus: 3 };
        assert!(!sel.is_collapsed());
        assert_eq!(sel.range(), (3, 10));
    }

    #[test]
    fn selected_text_extracts_range() {
        let sel = TextSelection { anchor: 0, focus: 5 };
        assert_eq!(sel.selected_text("Hello, world!"), "Hello");
    }

    #[test]
    fn selected_text_backward() {
        let sel = TextSelection { anchor: 7, focus: 0 };
        assert_eq!(sel.selected_text("Hello, world!"), "Hello, ");
    }

    #[test]
    fn collapsed_selected_text_is_empty() {
        let sel = TextSelection::collapsed(3);
        assert_eq!(sel.selected_text("Hello"), "");
    }
}
```

**Step 2: Implement selection.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affinity {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPosition {
    pub index: usize,
    pub affinity: Affinity,
}

impl TextPosition {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            affinity: Affinity::Before,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl TextSelection {
    pub fn collapsed(index: usize) -> Self {
        Self {
            anchor: index,
            focus: index,
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }

    pub fn range(&self) -> (usize, usize) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    pub fn selected_text<'a>(&self, source: &'a str) -> &'a str {
        let (start, end) = self.range();
        let start = start.min(source.len());
        let end = end.min(source.len());
        &source[start..end]
    }
}

impl Default for TextSelection {
    fn default() -> Self {
        Self::collapsed(0)
    }
}
```

**Step 3: Update lib.rs**

Add `mod selection;` and:
```rust
pub use selection::{Affinity, TextPosition, TextSelection};
```

**Step 4: Run tests**

Run: `cargo test -p velox-text`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add TextSelection, TextPosition, Affinity types"
```

---

## Task 5: UndoStack with Coalescing

**Files:**
- Create: `crates/velox-text/src/undo.rs`
- Modify: `crates/velox-text/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_undo() {
        let mut stack = UndoStack::new();
        stack.push(EditCommand::Insert {
            position: 0,
            text: "hello".into(),
        });
        let cmd = stack.undo();
        assert!(cmd.is_some());
        match cmd.unwrap() {
            EditCommand::Insert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "hello");
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn undo_then_redo() {
        let mut stack = UndoStack::new();
        stack.push(EditCommand::Insert {
            position: 0,
            text: "hi".into(),
        });
        stack.undo();
        let cmd = stack.redo();
        assert!(cmd.is_some());
    }

    #[test]
    fn push_after_undo_clears_redo() {
        let mut stack = UndoStack::new();
        stack.push(EditCommand::Insert {
            position: 0,
            text: "a".into(),
        });
        stack.push(EditCommand::Insert {
            position: 1,
            text: "b".into(),
        });
        stack.undo();
        stack.push(EditCommand::Insert {
            position: 1,
            text: "c".into(),
        });
        let redo = stack.redo();
        assert!(redo.is_none());
    }

    #[test]
    fn coalesce_consecutive_inserts() {
        let mut stack = UndoStack::new();
        stack.push_coalesced(EditCommand::Insert {
            position: 0,
            text: "h".into(),
        });
        stack.push_coalesced(EditCommand::Insert {
            position: 1,
            text: "e".into(),
        });
        stack.push_coalesced(EditCommand::Insert {
            position: 2,
            text: "l".into(),
        });
        let cmd = stack.undo();
        assert!(cmd.is_some());
        match cmd.unwrap() {
            EditCommand::Insert { position, text } => {
                assert_eq!(position, 0);
                assert_eq!(text, "hel");
            }
            _ => panic!("expected coalesced Insert"),
        }
        assert!(stack.undo().is_none());
    }

    #[test]
    fn delete_breaks_coalescing() {
        let mut stack = UndoStack::new();
        stack.push_coalesced(EditCommand::Insert {
            position: 0,
            text: "a".into(),
        });
        stack.push(EditCommand::Delete {
            position: 0,
            text: "a".into(),
        });
        assert!(stack.undo().is_some());
        assert!(stack.undo().is_some());
        assert!(stack.undo().is_none());
    }

    #[test]
    fn empty_undo_returns_none() {
        let mut stack = UndoStack::new();
        assert!(stack.undo().is_none());
    }

    #[test]
    fn stack_cap() {
        let mut stack = UndoStack::new();
        for i in 0..150 {
            stack.push(EditCommand::Insert {
                position: i,
                text: "x".into(),
            });
        }
        let mut count = 0;
        while stack.undo().is_some() {
            count += 1;
        }
        assert!(count <= 100);
    }
}
```

**Step 2: Implement undo.rs**

```rust
const MAX_UNDO_DEPTH: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    Insert { position: usize, text: String },
    Delete { position: usize, text: String },
    Replace { position: usize, old: String, new: String },
}

pub struct UndoStack {
    undo: Vec<EditCommand>,
    redo: Vec<EditCommand>,
}

impl UndoStack {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: EditCommand) {
        self.redo.clear();
        self.undo.push(cmd);
        if self.undo.len() > MAX_UNDO_DEPTH {
            self.undo.remove(0);
        }
    }

    pub fn push_coalesced(&mut self, cmd: EditCommand) {
        self.redo.clear();
        if let EditCommand::Insert { position, ref text } = cmd {
            if let Some(EditCommand::Insert {
                position: prev_pos,
                text: ref mut prev_text,
            }) = self.undo.last_mut()
            {
                if *prev_pos + prev_text.len() == position {
                    prev_text.push_str(text);
                    return;
                }
            }
        }
        self.undo.push(cmd);
        if self.undo.len() > MAX_UNDO_DEPTH {
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self) -> Option<EditCommand> {
        let cmd = self.undo.pop()?;
        self.redo.push(cmd.clone());
        Some(cmd)
    }

    pub fn redo(&mut self) -> Option<EditCommand> {
        let cmd = self.redo.pop()?;
        self.undo.push(cmd.clone());
        Some(cmd)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Update lib.rs**

Add `mod undo;` and:
```rust
pub use undo::{EditCommand, UndoStack};
```

**Step 4: Run tests**

Run: `cargo test -p velox-text`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add UndoStack with hybrid coalescing for edit commands"
```

---

## Task 6: EditableText Core (insert, delete, cursor)

**Files:**
- Create: `crates/velox-text/src/editable.rs`
- Modify: `crates/velox-text/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_system::FontSystem;

    fn make_editable(fs: &mut FontSystem, text: &str) -> EditableText {
        let mut e = EditableText::new(fs, 14.0, 20.0, false);
        e.set_size(fs, 400.0, 100.0);
        if !text.is_empty() {
            e.set_text(fs, text);
        }
        e
    }

    #[test]
    fn insert_char() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "");
        e.insert_char(&mut fs, 'H');
        e.insert_char(&mut fs, 'i');
        assert_eq!(e.text(), "Hi");
        assert_eq!(e.selection().focus, 2);
    }

    #[test]
    fn delete_backward() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 5);
        e.delete_backward(&mut fs);
        assert_eq!(e.text(), "Hell");
    }

    #[test]
    fn delete_forward() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 0);
        e.delete_forward(&mut fs);
        assert_eq!(e.text(), "ello");
    }

    #[test]
    fn delete_selection() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello World");
        e.set_selection(TextSelection { anchor: 5, focus: 11 });
        e.delete_backward(&mut fs);
        assert_eq!(e.text(), "Hello");
    }

    #[test]
    fn insert_replaces_selection() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello World");
        e.set_selection(TextSelection { anchor: 0, focus: 5 });
        e.insert_char(&mut fs, 'Y');
        assert_eq!(e.text(), "Y World");
    }

    #[test]
    fn select_all() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.select_all();
        let sel = e.selection();
        assert_eq!(sel.anchor, 0);
        assert_eq!(sel.focus, 5);
    }

    #[test]
    fn insert_text_paste() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "AB");
        e.move_cursor_to(&mut fs, 1);
        e.insert_text(&mut fs, "xyz");
        assert_eq!(e.text(), "AxyzB");
        assert_eq!(e.selection().focus, 4);
    }

    #[test]
    fn undo_insert() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "");
        e.insert_char(&mut fs, 'A');
        e.insert_char(&mut fs, 'B');
        e.undo(&mut fs);
        assert_eq!(e.text(), "");
    }

    #[test]
    fn undo_then_redo() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "");
        e.insert_char(&mut fs, 'A');
        e.insert_char(&mut fs, 'B');
        e.undo(&mut fs);
        e.redo(&mut fs);
        assert_eq!(e.text(), "AB");
    }

    #[test]
    fn move_cursor_left_right() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 3);
        e.move_cursor(&mut fs, CursorDirection::Left, false);
        assert_eq!(e.selection().focus, 2);
        e.move_cursor(&mut fs, CursorDirection::Right, false);
        assert_eq!(e.selection().focus, 3);
    }

    #[test]
    fn move_cursor_with_shift_extends_selection() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 2);
        e.move_cursor(&mut fs, CursorDirection::Right, true);
        e.move_cursor(&mut fs, CursorDirection::Right, true);
        let sel = e.selection();
        assert_eq!(sel.anchor, 2);
        assert_eq!(sel.focus, 4);
    }

    #[test]
    fn home_end() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 3);
        e.move_cursor(&mut fs, CursorDirection::Home, false);
        assert_eq!(e.selection().focus, 0);
        e.move_cursor(&mut fs, CursorDirection::End, false);
        assert_eq!(e.selection().focus, 5);
    }

    #[test]
    fn cursor_clamps_at_boundaries() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hi");
        e.move_cursor_to(&mut fs, 0);
        e.move_cursor(&mut fs, CursorDirection::Left, false);
        assert_eq!(e.selection().focus, 0);
        e.move_cursor_to(&mut fs, 2);
        e.move_cursor(&mut fs, CursorDirection::Right, false);
        assert_eq!(e.selection().focus, 2);
    }
}
```

**Step 2: Implement editable.rs**

```rust
use crate::attrs::TextAttrs;
use crate::buffer::TextBuffer;
use crate::font_system::FontSystem;
use crate::selection::TextSelection;
use crate::undo::{EditCommand, UndoStack};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

pub struct EditableText {
    text: String,
    buffer: TextBuffer,
    selection: TextSelection,
    undo_stack: UndoStack,
    attrs: TextAttrs,
    multiline: bool,
}

impl EditableText {
    pub fn new(font_system: &mut FontSystem, font_size: f32, line_height: f32, multiline: bool) -> Self {
        Self {
            text: String::new(),
            buffer: TextBuffer::new(font_system, font_size, line_height),
            selection: TextSelection::default(),
            undo_stack: UndoStack::new(),
            attrs: TextAttrs::default(),
            multiline,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn selection(&self) -> TextSelection {
        self.selection
    }

    pub fn set_selection(&mut self, sel: TextSelection) {
        self.selection = sel;
    }

    pub fn set_size(&mut self, font_system: &mut FontSystem, width: f32, height: f32) {
        self.buffer.set_size(font_system, width, height);
    }

    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str) {
        self.text = text.to_owned();
        self.selection = TextSelection::collapsed(self.text.len());
        self.undo_stack.clear();
        self.reshape(font_system);
    }

    pub fn select_all(&mut self) {
        self.selection = TextSelection {
            anchor: 0,
            focus: self.text.len(),
        };
    }

    pub fn selected_text(&self) -> &str {
        self.selection.selected_text(&self.text)
    }

    pub fn insert_char(&mut self, font_system: &mut FontSystem, ch: char) {
        self.delete_selection_inner(font_system);
        let pos = self.selection.focus;
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.text.insert_str(pos, s);
        self.selection = TextSelection::collapsed(pos + s.len());
        self.undo_stack.push_coalesced(EditCommand::Insert {
            position: pos,
            text: s.to_owned(),
        });
        self.reshape(font_system);
    }

    pub fn insert_text(&mut self, font_system: &mut FontSystem, text: &str) {
        self.delete_selection_inner(font_system);
        let pos = self.selection.focus;
        self.text.insert_str(pos, text);
        self.selection = TextSelection::collapsed(pos + text.len());
        self.undo_stack.push(EditCommand::Insert {
            position: pos,
            text: text.to_owned(),
        });
        self.reshape(font_system);
    }

    pub fn delete_backward(&mut self, font_system: &mut FontSystem) {
        if !self.selection.is_collapsed() {
            self.delete_selection_inner(font_system);
            self.reshape(font_system);
            return;
        }
        let pos = self.selection.focus;
        if pos == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.text, pos);
        let deleted = self.text[prev..pos].to_owned();
        self.text.replace_range(prev..pos, "");
        self.selection = TextSelection::collapsed(prev);
        self.undo_stack.push(EditCommand::Delete {
            position: prev,
            text: deleted,
        });
        self.reshape(font_system);
    }

    pub fn delete_forward(&mut self, font_system: &mut FontSystem) {
        if !self.selection.is_collapsed() {
            self.delete_selection_inner(font_system);
            self.reshape(font_system);
            return;
        }
        let pos = self.selection.focus;
        if pos >= self.text.len() {
            return;
        }
        let next = next_char_boundary(&self.text, pos);
        let deleted = self.text[pos..next].to_owned();
        self.text.replace_range(pos..next, "");
        self.undo_stack.push(EditCommand::Delete {
            position: pos,
            text: deleted,
        });
        self.reshape(font_system);
    }

    pub fn move_cursor_to(&mut self, _font_system: &mut FontSystem, index: usize) {
        let clamped = index.min(self.text.len());
        self.selection = TextSelection::collapsed(clamped);
    }

    pub fn move_cursor(
        &mut self,
        _font_system: &mut FontSystem,
        direction: CursorDirection,
        extend_selection: bool,
    ) {
        let pos = self.selection.focus;
        let new_pos = match direction {
            CursorDirection::Left => {
                if pos == 0 {
                    0
                } else {
                    prev_char_boundary(&self.text, pos)
                }
            }
            CursorDirection::Right => {
                if pos >= self.text.len() {
                    self.text.len()
                } else {
                    next_char_boundary(&self.text, pos)
                }
            }
            CursorDirection::Home => 0,
            CursorDirection::End => self.text.len(),
            CursorDirection::Up | CursorDirection::Down => pos,
        };

        if extend_selection {
            self.selection.focus = new_pos;
        } else {
            self.selection = TextSelection::collapsed(new_pos);
        }
    }

    pub fn undo(&mut self, font_system: &mut FontSystem) {
        let Some(cmd) = self.undo_stack.undo() else {
            return;
        };
        match cmd {
            EditCommand::Insert { position, ref text } => {
                self.text.replace_range(position..position + text.len(), "");
                self.selection = TextSelection::collapsed(position);
            }
            EditCommand::Delete { position, ref text } => {
                self.text.insert_str(position, text);
                self.selection = TextSelection::collapsed(position + text.len());
            }
            EditCommand::Replace {
                position,
                ref old,
                ref new,
            } => {
                self.text.replace_range(position..position + new.len(), old);
                self.selection = TextSelection::collapsed(position + old.len());
            }
        }
        self.reshape(font_system);
    }

    pub fn redo(&mut self, font_system: &mut FontSystem) {
        let Some(cmd) = self.undo_stack.redo() else {
            return;
        };
        match cmd {
            EditCommand::Insert { position, ref text } => {
                self.text.insert_str(position, text);
                self.selection = TextSelection::collapsed(position + text.len());
            }
            EditCommand::Delete { position, ref text } => {
                self.text.replace_range(position..position + text.len(), "");
                self.selection = TextSelection::collapsed(position);
            }
            EditCommand::Replace {
                position,
                ref old,
                ref new,
            } => {
                self.text.replace_range(position..position + old.len(), new);
                self.selection = TextSelection::collapsed(position + new.len());
            }
        }
        self.reshape(font_system);
    }

    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    fn delete_selection_inner(&mut self, _font_system: &mut FontSystem) {
        if self.selection.is_collapsed() {
            return;
        }
        let (start, end) = self.selection.range();
        let deleted = self.text[start..end].to_owned();
        self.text.replace_range(start..end, "");
        self.selection = TextSelection::collapsed(start);
        self.undo_stack.push(EditCommand::Delete {
            position: start,
            text: deleted,
        });
    }

    fn reshape(&mut self, font_system: &mut FontSystem) {
        self.buffer
            .set_text(font_system, &self.text, self.attrs.clone());
        self.buffer.shape(font_system);
    }
}

fn prev_char_boundary(text: &str, index: usize) -> usize {
    let mut i = index.saturating_sub(1);
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    let mut i = index + 1;
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i.min(text.len())
}
```

**Step 3: Update lib.rs**

Add `mod editable;` and:
```rust
pub use editable::{CursorDirection, EditableText};
```

**Step 4: Run tests**

Run: `cargo test -p velox-text`
Expected: all tests pass

**Step 5: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add EditableText with insert, delete, cursor, selection, undo/redo"
```

---

## Task 7: Hit Testing (pixel to text position)

**Files:**
- Create: `crates/velox-text/src/hit_test.rs`
- Modify: `crates/velox-text/src/editable.rs`
- Modify: `crates/velox-text/src/lib.rs`

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_system::FontSystem;

    #[test]
    fn hit_test_beginning() {
        let mut fs = FontSystem::new();
        let mut e = EditableText::new(&mut fs, 14.0, 20.0, false);
        e.set_size(&mut fs, 400.0, 100.0);
        e.set_text(&mut fs, "Hello World");
        let pos = e.hit_test(&fs, 0.0, 10.0);
        assert_eq!(pos, 0);
    }

    #[test]
    fn hit_test_end() {
        let mut fs = FontSystem::new();
        let mut e = EditableText::new(&mut fs, 14.0, 20.0, false);
        e.set_size(&mut fs, 400.0, 100.0);
        e.set_text(&mut fs, "Hello World");
        let pos = e.hit_test(&fs, 999.0, 10.0);
        assert_eq!(pos, 11);
    }

    #[test]
    fn hit_test_negative_returns_zero() {
        let mut fs = FontSystem::new();
        let mut e = EditableText::new(&mut fs, 14.0, 20.0, false);
        e.set_size(&mut fs, 400.0, 100.0);
        e.set_text(&mut fs, "Hello");
        let pos = e.hit_test(&fs, -10.0, 10.0);
        assert_eq!(pos, 0);
    }
}
```

**Step 2: Add hit_test to EditableText**

In `editable.rs`, add method:

```rust
pub fn hit_test(&self, _font_system: &FontSystem, x: f32, y: f32) -> usize {
    if x < 0.0 {
        return 0;
    }
    for run in self.buffer.layout_runs() {
        if y >= run.line_y - run.line_height && y < run.line_y {
            let mut last_end = 0;
            for glyph in run.glyphs.iter() {
                let glyph_mid = glyph.x + glyph.w / 2.0;
                if x < glyph_mid {
                    return glyph.start;
                }
                last_end = glyph.end;
            }
            return last_end;
        }
    }
    self.text.len()
}
```

Note: `hit_test.rs` is not needed as a separate file — the method lives on EditableText. Remove the `hit_test.rs` create entry; just modify `editable.rs`.

**Step 3: Run tests**

Run: `cargo test -p velox-text`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add hit_test for pixel-to-text-position mapping"
```

---

## Task 8: Cursor and Selection Rects

**Files:**
- Modify: `crates/velox-text/src/editable.rs`

**Step 1: Write tests**

```rust
#[test]
fn cursor_rect_at_start() {
    let mut fs = FontSystem::new();
    let mut e = make_editable(&mut fs, "Hello");
    e.move_cursor_to(&mut fs, 0);
    let rect = e.cursor_rect();
    assert!(rect.is_some());
    let rect = rect.unwrap();
    assert!(rect.width > 0.0);
    assert!(rect.height > 0.0);
}

#[test]
fn selection_rects_when_selected() {
    let mut fs = FontSystem::new();
    let mut e = make_editable(&mut fs, "Hello World");
    e.set_selection(TextSelection { anchor: 0, focus: 5 });
    let rects = e.selection_rects();
    assert!(!rects.is_empty());
    assert!(rects[0].width > 0.0);
}

#[test]
fn selection_rects_empty_when_collapsed() {
    let mut fs = FontSystem::new();
    let mut e = make_editable(&mut fs, "Hello");
    e.move_cursor_to(&mut fs, 2);
    let rects = e.selection_rects();
    assert!(rects.is_empty());
}
```

**Step 2: Implement cursor_rect and selection_rects on EditableText**

Uses `velox_scene::Rect` — but velox-text doesn't depend on velox-scene. Use a local Rect-like return or add a minimal dependency. Simplest: return tuples `(x, y, w, h)` or define a simple struct in velox-text.

Actually, define a simple `TextRect` in velox-text to avoid cross-crate dependency:

```rust
#[derive(Debug, Clone, Copy)]
pub struct TextRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

Add to `editable.rs`:

```rust
pub fn cursor_rect(&self) -> Option<TextRect> {
    let pos = self.selection.focus;
    for run in self.buffer.layout_runs() {
        let line_top = run.line_y - run.line_height;
        for glyph in run.glyphs.iter() {
            if pos >= glyph.start && pos <= glyph.end {
                let x = if pos == glyph.start {
                    glyph.x
                } else {
                    glyph.x + glyph.w
                };
                return Some(TextRect {
                    x,
                    y: line_top,
                    width: 1.5,
                    height: run.line_height,
                });
            }
        }
        if run.glyphs.is_empty() || pos >= run.glyphs.last().map(|g| g.end).unwrap_or(0) {
            let x = run.glyphs.last().map(|g| g.x + g.w).unwrap_or(0.0);
            return Some(TextRect {
                x,
                y: line_top,
                width: 1.5,
                height: run.line_height,
            });
        }
    }
    Some(TextRect {
        x: 0.0,
        y: 0.0,
        width: 1.5,
        height: 20.0,
    })
}

pub fn selection_rects(&self) -> Vec<TextRect> {
    if self.selection.is_collapsed() {
        return Vec::new();
    }
    let (start, end) = self.selection.range();
    let mut rects = Vec::new();
    for run in self.buffer.layout_runs() {
        let line_top = run.line_y - run.line_height;
        let mut line_start_x = None;
        let mut line_end_x = None;
        for glyph in run.glyphs.iter() {
            if glyph.end <= start || glyph.start >= end {
                continue;
            }
            let gx_start = glyph.x;
            let gx_end = glyph.x + glyph.w;
            if line_start_x.is_none() {
                line_start_x = Some(gx_start);
            }
            line_end_x = Some(gx_end);
        }
        if let (Some(sx), Some(ex)) = (line_start_x, line_end_x) {
            rects.push(TextRect {
                x: sx,
                y: line_top,
                width: ex - sx,
                height: run.line_height,
            });
        }
    }
    rects
}
```

Export `TextRect` from lib.rs.

**Step 3: Run tests**

Run: `cargo test -p velox-text`
Expected: all tests pass

**Step 4: Commit**

```bash
git add crates/velox-text/
git commit -m "feat(text): add cursor_rect and selection_rects for rendering"
```

---

## Task 9: DrawGlyphs Paint Command

**Files:**
- Modify: `crates/velox-scene/src/paint.rs`
- Modify: `crates/velox-render/src/renderer.rs`

**Step 1: Add PositionedGlyph and DrawGlyphs to paint.rs**

```rust
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub cache_key: cosmic_text::CacheKey,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

Add variant to PaintCommand:
```rust
DrawGlyphs {
    glyphs: Vec<PositionedGlyph>,
    color: Color,
},
```

Add to CommandList:
```rust
pub fn draw_glyphs(&mut self, glyphs: Vec<PositionedGlyph>, color: Color) {
    self.commands.push(PaintCommand::DrawGlyphs { glyphs, color });
}
```

Add `cosmic-text = { workspace = true }` to `crates/velox-scene/Cargo.toml`.

Export `PositionedGlyph` from `lib.rs`.

**Step 2: Handle DrawGlyphs in Renderer**

In `crates/velox-render/src/renderer.rs`, update the match in `render()`:

```rust
PaintCommand::DrawGlyphs { glyphs, color } => {
    let c = color_to_f32(color);
    for glyph in glyphs {
        if let Some(region) = atlas.get(&glyph.cache_key) {
            let uv = atlas.uv(&region);
            glyph_quads.push(GlyphQuad {
                x: glyph.x,
                y: glyph.y,
                width: glyph.width,
                height: glyph.height,
                uv,
                color: c,
            });
        }
    }
}
```

Add `let mut glyph_quads = Vec::new();` alongside the `rects` vec, and `self.glyph_renderer.prepare(...)` before the render pass.

**Step 3: Verify**

Run: `cargo build --workspace`
Expected: compiles

**Step 4: Commit**

```bash
git add crates/velox-scene/ crates/velox-render/
git commit -m "feat: add DrawGlyphs paint command and renderer support"
```

---

## Task 10: Wire Keyboard Events in VeloxHandler

**Files:**
- Modify: `crates/velox-app/src/handler.rs`

**Step 1: Add winit key conversion helper**

Create a helper function to convert winit `keyboard::Key` + modifiers to velox `Key` + `Modifiers`:

```rust
fn convert_winit_key(key: &winit::keyboard::Key) -> Option<crate::shortcut_key_from_winit(key)> {
    // See implementation below
}
```

Actually, add a new file `crates/velox-app/src/key_convert.rs` with:

```rust
use velox_scene::shortcut::{Key, Modifiers};
use winit::event::ElementState;
use winit::keyboard::{Key as WinitKey, NamedKey, ModifiersState};

use velox_scene::event::KeyState;

pub fn convert_key(winit_key: &WinitKey) -> Option<Key> {
    match winit_key {
        WinitKey::Character(c) => {
            let ch = c.chars().next()?;
            match ch.to_ascii_lowercase() {
                'a' => Some(Key::A), 'b' => Some(Key::B), 'c' => Some(Key::C),
                'd' => Some(Key::D), 'e' => Some(Key::E), 'f' => Some(Key::F),
                'g' => Some(Key::G), 'h' => Some(Key::H), 'i' => Some(Key::I),
                'j' => Some(Key::J), 'k' => Some(Key::K), 'l' => Some(Key::L),
                'm' => Some(Key::M), 'n' => Some(Key::N), 'o' => Some(Key::O),
                'p' => Some(Key::P), 'q' => Some(Key::Q), 'r' => Some(Key::R),
                's' => Some(Key::S), 't' => Some(Key::T), 'u' => Some(Key::U),
                'v' => Some(Key::V), 'w' => Some(Key::W), 'x' => Some(Key::X),
                'y' => Some(Key::Y), 'z' => Some(Key::Z),
                '0' => Some(Key::Num0), '1' => Some(Key::Num1), '2' => Some(Key::Num2),
                '3' => Some(Key::Num3), '4' => Some(Key::Num4), '5' => Some(Key::Num5),
                '6' => Some(Key::Num6), '7' => Some(Key::Num7), '8' => Some(Key::Num8),
                '9' => Some(Key::Num9),
                _ => None,
            }
        }
        WinitKey::Named(named) => match named {
            NamedKey::Enter => Some(Key::Enter),
            NamedKey::Escape => Some(Key::Escape),
            NamedKey::Tab => Some(Key::Tab),
            NamedKey::Space => Some(Key::Space),
            NamedKey::Backspace => Some(Key::Backspace),
            NamedKey::Delete => Some(Key::Delete),
            NamedKey::ArrowUp => Some(Key::ArrowUp),
            NamedKey::ArrowDown => Some(Key::ArrowDown),
            NamedKey::ArrowLeft => Some(Key::ArrowLeft),
            NamedKey::ArrowRight => Some(Key::ArrowRight),
            NamedKey::Home => Some(Key::Home),
            NamedKey::End => Some(Key::End),
            NamedKey::PageUp => Some(Key::PageUp),
            NamedKey::PageDown => Some(Key::PageDown),
            NamedKey::F1 => Some(Key::F1), NamedKey::F2 => Some(Key::F2),
            NamedKey::F3 => Some(Key::F3), NamedKey::F4 => Some(Key::F4),
            NamedKey::F5 => Some(Key::F5), NamedKey::F6 => Some(Key::F6),
            NamedKey::F7 => Some(Key::F7), NamedKey::F8 => Some(Key::F8),
            NamedKey::F9 => Some(Key::F9), NamedKey::F10 => Some(Key::F10),
            NamedKey::F11 => Some(Key::F11), NamedKey::F12 => Some(Key::F12),
            _ => None,
        },
        _ => None,
    }
}

pub fn convert_modifiers(mods: ModifiersState) -> Modifiers {
    let mut result = Modifiers::empty();
    if mods.shift_key() {
        result |= Modifiers::SHIFT;
    }
    if mods.control_key() {
        result |= Modifiers::CTRL;
    }
    if mods.alt_key() {
        result |= Modifiers::ALT;
    }
    if mods.super_key() {
        result |= Modifiers::SUPER;
    }
    result
}

pub fn convert_element_state(state: ElementState) -> KeyState {
    match state {
        ElementState::Pressed => KeyState::Pressed,
        ElementState::Released => KeyState::Released,
    }
}
```

**Step 2: Add KeyboardInput handling to VeloxHandler**

In `handler.rs`, add to the `window_event` match:

```rust
WindowEvent::KeyboardInput { event, .. } => {
    if !event.state.is_pressed() {
        return;
    }
    let Some(velox_key) = key_convert::convert_key(&event.logical_key) else {
        return;
    };
    let modifiers = key_convert::convert_modifiers(self.current_modifiers);

    if self.shortcuts.handle_key_event(velox_key, modifiers) {
        return;
    }

    if let Some(ws) = self.windows.get_mut(&velox_id) {
        if let Some(focused) = ws.scene.focus().focused() {
            let text = event.text.as_ref().map(|t| t.to_string());
            let key_event = velox_scene::KeyEvent {
                key: velox_key,
                modifiers,
                state: key_convert::convert_element_state(event.state),
                text,
            };
            ws.scene.tree_mut().dispatch_key_event(focused, &key_event);
        }
    }
}
WindowEvent::ModifiersChanged(mods) => {
    self.current_modifiers = mods.state();
}
```

Add `current_modifiers: winit::keyboard::ModifiersState` field to VeloxHandler, initialized with `ModifiersState::default()`.

Add `mod key_convert;` to `crates/velox-app/src/lib.rs` or handler.rs.

**Step 3: Verify**

Run: `cargo build --workspace`
Expected: compiles

**Step 4: Commit**

```bash
git add crates/velox-app/
git commit -m "feat(app): wire keyboard events through shortcuts and focused node dispatch"
```

---

## Task 11: Wire Mouse Events and Clipboard

**Files:**
- Modify: `crates/velox-app/Cargo.toml` (add arboard)
- Modify: `crates/velox-app/src/handler.rs`
- Modify: `Cargo.toml` (workspace dep)

**Step 1: Add arboard dependency**

Workspace root Cargo.toml:
```toml
arboard = "3"
```

velox-app Cargo.toml:
```toml
arboard = { workspace = true }
```

**Step 2: Add mouse tracking and clipboard to VeloxHandler**

Add fields:
```rust
cursor_position: Point,
clipboard: Option<arboard::Clipboard>,
```

Initialize clipboard in `resumed()`:
```rust
self.clipboard = arboard::Clipboard::new().ok();
```

Add mouse event handling:
```rust
WindowEvent::CursorMoved { position, .. } => {
    self.cursor_position = velox_scene::Point::new(position.x as f32, position.y as f32);
}
WindowEvent::MouseInput { state, button, .. } => {
    if state == winit::event::ElementState::Pressed
        && button == winit::event::MouseButton::Left
    {
        if let Some(ws) = self.windows.get_mut(&velox_id) {
            let point = self.cursor_position;
            if let Some(hit_id) = ws.scene.hit_test(point) {
                ws.scene.focus_mut().request_focus(hit_id);
                let node_rect = ws.scene.tree().rect(hit_id).unwrap_or(velox_scene::Rect::zero());
                let local_pos = velox_scene::Point::new(
                    point.x - node_rect.x,
                    point.y - node_rect.y,
                );
                let mouse_event = velox_scene::MouseEvent {
                    position: local_pos,
                    button: velox_scene::MouseButton::Left,
                    state: velox_scene::ButtonState::Pressed,
                    click_count: 1,
                    modifiers: key_convert::convert_modifiers(self.current_modifiers),
                };
                ws.scene.tree_mut().dispatch_mouse_event(hit_id, &mouse_event);
            }
        }
    }
}
```

**Step 3: Verify**

Run: `cargo build --workspace`
Expected: compiles

**Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crates/velox-app/
git commit -m "feat(app): wire mouse events, focus, and clipboard via arboard"
```

---

## Task 12: Update Facade and Phase 4 Demo

**Files:**
- Modify: `crates/velox/src/lib.rs`
- Create: `crates/velox/examples/phase4_demo.rs`
- Modify: `CLAUDE.md`

**Step 1: Update facade prelude**

Add to prelude:
```rust
pub use velox_scene::{EventHandler, EventContext, KeyEvent, MouseEvent};
pub use velox_text::{EditableText, TextSelection};
```

**Step 2: Create phase4_demo.rs**

A demo with a text input field that responds to keyboard and mouse:

```rust
use std::cell::Cell;
use velox::prelude::*;
use velox::scene::{
    ButtonState, Color, CommandList, EventContext, EventHandler, KeyEvent,
    KeyState, MouseButton, MouseEvent, PaddingLayout, Painter, PositionedGlyph,
};
use velox::text::{
    CursorDirection, EditableText, FontSystem, GlyphRasterizer, TextRect, TextSelection,
};
use velox::scene::shortcut::{Key, Modifiers};

struct TextInputWidget {
    editable: EditableText,
    font_system: FontSystem,
    rasterizer: GlyphRasterizer,
    focused: bool,
    cursor_visible: bool,
}

impl TextInputWidget {
    fn new() -> Self {
        let mut fs = FontSystem::new();
        let mut editable = EditableText::new(&mut fs, 16.0, 24.0, false);
        editable.set_size(&mut fs, 500.0, 40.0);
        editable.set_text(&mut fs, "Type here...");
        editable.select_all();
        Self {
            editable,
            font_system: fs,
            rasterizer: GlyphRasterizer::new(),
            focused: true,
            cursor_visible: true,
        }
    }
}

impl EventHandler for TextInputWidget {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool {
        if !event.state.is_pressed() {
            return false;
        }
        let is_cmd = cfg!(target_os = "macos") && event.modifiers.contains(Modifiers::SUPER)
            || !cfg!(target_os = "macos") && event.modifiers.contains(Modifiers::CTRL);

        match event.key {
            Key::A if is_cmd => self.editable.select_all(),
            Key::Z if is_cmd && event.modifiers.contains(Modifiers::SHIFT) => {
                self.editable.redo(&mut self.font_system);
            }
            Key::Z if is_cmd => self.editable.undo(&mut self.font_system),
            Key::C if is_cmd => {
                let text = self.editable.selected_text().to_owned();
                if !text.is_empty() {
                    ctx.clipboard_set(&text);
                }
            }
            Key::X if is_cmd => {
                let text = self.editable.selected_text().to_owned();
                if !text.is_empty() {
                    ctx.clipboard_set(&text);
                    self.editable.delete_backward(&mut self.font_system);
                }
            }
            Key::V if is_cmd => {
                if let Some(text) = ctx.clipboard_get() {
                    let text = text.to_owned();
                    self.editable.insert_text(&mut self.font_system, &text);
                }
            }
            Key::Backspace => self.editable.delete_backward(&mut self.font_system),
            Key::Delete => self.editable.delete_forward(&mut self.font_system),
            Key::ArrowLeft => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                self.editable.move_cursor(&mut self.font_system, CursorDirection::Left, extend);
            }
            Key::ArrowRight => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                self.editable.move_cursor(&mut self.font_system, CursorDirection::Right, extend);
            }
            Key::Home => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                self.editable.move_cursor(&mut self.font_system, CursorDirection::Home, extend);
            }
            Key::End => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                self.editable.move_cursor(&mut self.font_system, CursorDirection::End, extend);
            }
            _ => {
                if let Some(ref text) = event.text {
                    for ch in text.chars() {
                        if !ch.is_control() {
                            self.editable.insert_char(&mut self.font_system, ch);
                        }
                    }
                } else {
                    return false;
                }
            }
        }
        self.cursor_visible = true;
        ctx.request_redraw();
        true
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
            let pos = self.editable.hit_test(&self.font_system, event.position.x, event.position.y);
            self.editable.move_cursor_to(&mut self.font_system, pos);
            self.cursor_visible = true;
            ctx.request_redraw();
            return true;
        }
        false
    }

    fn handle_focus(&mut self, gained: bool) {
        self.focused = gained;
        self.cursor_visible = gained;
    }
}

impl Painter for TextInputWidget {
    fn paint(&self, rect: Rect, commands: &mut CommandList) {
        commands.fill_rect(rect, Color::rgb(50, 50, 60));

        for sel_rect in self.editable.selection_rects() {
            commands.fill_rect(
                Rect::new(rect.x + sel_rect.x, rect.y + sel_rect.y, sel_rect.width, sel_rect.height),
                Color::rgba(80, 120, 200, 100),
            );
        }

        let mut glyphs = Vec::new();
        for run in self.editable.buffer().layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                glyphs.push(PositionedGlyph {
                    cache_key: physical.cache_key,
                    x: rect.x + physical.x as f32,
                    y: rect.y + run.line_y - run.line_height + physical.y as f32,
                    width: glyph.w,
                    height: run.line_height,
                });
            }
        }
        if !glyphs.is_empty() {
            commands.draw_glyphs(glyphs, Color::rgb(230, 230, 240));
        }

        if self.focused && self.cursor_visible {
            if let Some(cr) = self.editable.cursor_rect() {
                commands.fill_rect(
                    Rect::new(rect.x + cr.x, rect.y + cr.y, cr.width, cr.height),
                    Color::rgb(200, 200, 220),
                );
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Phase 4 Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — Text Editing")
                .size(800, 400),
        )
        .setup(|scene| {
            let root = scene.tree_mut().insert(None);
            scene.tree_mut().set_rect(root, Rect::new(0.0, 0.0, 800.0, 400.0));
            scene.tree_mut().set_layout(root, PaddingLayout {
                top: 50.0, right: 50.0, bottom: 50.0, left: 50.0,
            });

            let input = scene.tree_mut().insert(Some(root));
            scene.tree_mut().set_rect(input, Rect::new(0.0, 0.0, 700.0, 40.0));

            let widget = TextInputWidget::new();
            scene.tree_mut().set_painter(input, widget);

            scene.focus_mut().request_focus(input);
        })
        .run()
}
```

**Note:** The `TextInputWidget` must implement BOTH `Painter` and `EventHandler`. Since a node can only have one of each, and they're separate trait objects, you'll need to store the widget shared state (EditableText, etc.) in a way both can access. The simplest approach: make the widget implement `Painter`, and set it as the painter. For the event handler, create a thin wrapper that shares state via `Rc<RefCell<...>>`. This is the integration challenge — resolve it in the demo by having the same struct implement both traits and cloning state as needed. OR, set the event handler separately from the painter.

Actually, the cleanest approach for the demo: the `TextInputWidget` implements both traits. But Rust doesn't allow a single struct to be both `Box<dyn Painter>` and `Box<dyn EventHandler>` on the same node (they're separate fields). So:

- Store the widget in `Rc<RefCell<TextInputWidget>>`
- Create thin `TextInputPainter` and `TextInputEventHandler` wrappers that hold `Rc<RefCell<TextInputWidget>>`
- Set both on the node

This pattern is important and should be shown in the demo.

**Step 3: Update CLAUDE.md**

Update status to Phase 4 complete. Add velox-text editable text, event routing to implemented features.

**Step 4: Run full verification**

Run: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace`

**Step 5: Commit**

```bash
git add crates/velox/ CLAUDE.md
git commit -m "feat: Phase 4 complete — text editing, event routing, clipboard"
```

---

## Dependency Order

```
Wave A (parallel):
  - Task 1-2: Event types + EventHandler trait (velox-scene)
  - Task 4-5: TextSelection + UndoStack (velox-text)

Wave B (depends on Wave A):
  - Task 3: EventHandler on NodeTree (velox-scene, depends on Tasks 1-2)
  - Task 6: EditableText core (velox-text, depends on Tasks 4-5)

Wave C (depends on Wave B):
  - Task 7: Hit testing (velox-text)
  - Task 8: Cursor/selection rects (velox-text)
  - Task 9: DrawGlyphs paint command (velox-scene + velox-render)

Wave D (depends on Wave C):
  - Task 10: Wire keyboard events (velox-app)
  - Task 11: Wire mouse events + clipboard (velox-app)

Wave E (depends on Wave D):
  - Task 12: Facade update + demo
```
