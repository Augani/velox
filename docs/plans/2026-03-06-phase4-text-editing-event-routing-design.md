# Phase 4: Text Editing, Event Routing, and Commands — Design

## Overview

Phase 4 adds event routing from winit through the scene to focused nodes, an editable text model with cursor/selection/undo, clipboard integration, and a text input widget. This completes the deferred items from Phase 3 (minus IME, which is deferred to a later phase).

**Scope:** Event dispatching, editable text with cursor/selection, undo/redo with coalescing, clipboard (copy/cut/paste), hit-test-to-position, multi-line support.
**Deferred:** IME/preedit, rich text formatting, spellcheck.

## Event Routing (`velox-scene`)

### New trait: `EventHandler`

```rust
pub trait EventHandler: 'static {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool;
    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool { false }
    fn handle_focus(&mut self, gained: bool) {}
}
```

- `KeyEvent` — key, modifiers, state (pressed/released), text (the character if printable)
- `MouseEvent` — position (local to node rect), button, state, click count
- `EventContext` — provides access to the node's rect, request_redraw, clipboard
- Returns `true` = consumed, `false` = bubble up

### Event dispatch flow

1. winit `KeyboardInput` → `VeloxHandler`
2. `ShortcutRegistry::handle_key_event()` — global shortcuts checked first
3. If not consumed → `FocusState::focused()` → get that node's `EventHandler` → call `handle_key()`
4. Mouse events: `hit_test()` to find target node → call `handle_mouse()`

### NodeTree additions

- `set_event_handler(id, impl EventHandler)` — same take/put-back pattern as Painter
- `dispatch_key_event(focus: NodeId, event: &KeyEvent, ctx: &mut EventContext) -> bool`

## Editable Text Model (`velox-text`)

### TextPosition

Represents a location in text:

```rust
pub struct TextPosition {
    pub index: usize,       // byte offset in the text string
    pub affinity: Affinity, // Left or Right (for line-wrap ambiguity)
}
```

### TextSelection

Cursor or range:

```rust
pub struct TextSelection {
    pub anchor: usize, // byte offset where selection started
    pub focus: usize,  // byte offset where cursor is (can equal anchor = no selection)
}
```

- `is_collapsed()` — true if anchor == focus (just a cursor)
- `range()` — returns ordered (start, end)
- `selected_text(source: &str)` — extracts the selected substring

### EditableText

Extends TextBuffer with editing state:

```rust
pub struct EditableText {
    text: String,
    buffer: TextBuffer,
    selection: TextSelection,
    undo_stack: UndoStack,
    multiline: bool,
}
```

Key methods:

- `insert_char(font_system, ch)` — insert at cursor, push to undo stack (coalesces consecutive char inserts)
- `delete_forward(font_system)` / `delete_backward(font_system)` — delete char/selection
- `move_cursor(direction, extend_selection)` — left/right/up/down/home/end, with shift = extend selection
- `select_all()`
- `select_word_at(index)` — for double-click
- `hit_test(x, y) -> TextPosition` — pixel coordinate to text position using cosmic-text layout data
- `cursor_rect() -> Rect` — returns the cursor's pixel rectangle for rendering
- `selection_rects() -> Vec<Rect>` — returns rectangles for selection highlight rendering
- `text()` / `set_text(font_system, text)` — get/replace full text
- `selected_text()` — get selected text for clipboard
- `insert_text(font_system, text)` — for paste operations
- `undo(font_system)` / `redo(font_system)` — undo/redo with reshaping

### Undo Stack (hybrid coalescing)

```rust
enum EditCommand {
    Insert { position: usize, text: String },
    Delete { position: usize, text: String },
    Replace { position: usize, old: String, new: String },
}
```

- Consecutive `Insert` commands at adjacent positions merge into one entry
- Any cursor movement, delete, or non-adjacent insert breaks the coalescing group
- Stack capped at a reasonable depth (e.g., 100 entries)

## Clipboard Integration (`velox-scene`)

### EventContext

Passed to event handlers:

```rust
pub struct EventContext<'a> {
    pub rect: Rect,
    clipboard: &'a mut Clipboard,
    redraw_requested: bool,
}
```

- `clipboard_get() -> Option<String>` — read text from clipboard
- `clipboard_set(text: &str)` — write text to clipboard
- `request_redraw()` — mark window as needing repaint

### Clipboard

Wraps `arboard::Clipboard`:

```rust
pub struct Clipboard {
    inner: arboard::Clipboard,
}
```

Owned by VeloxHandler, passed into event dispatch. Simple get/set text.

### Command dispatch

Standard text editing commands are handled by the EventHandler on the text input node:

- Cmd/Ctrl+C → copy selected text to clipboard
- Cmd/Ctrl+X → cut (copy + delete)
- Cmd/Ctrl+V → paste from clipboard
- Cmd/Ctrl+Z → undo
- Cmd/Ctrl+Shift+Z → redo
- Cmd/Ctrl+A → select all

Global shortcuts (via ShortcutRegistry) get first priority; text field shortcuts are handled locally.

## Text Input Node & Rendering

### TextInputHandler

An `EventHandler` implementation that owns an `EditableText` and handles:

- Key pressed with printable char → `editable.insert_char()`
- Backspace/Delete → `editable.delete_backward/forward()`
- Arrow keys (± shift) → `editable.move_cursor()`
- Cmd+C/X/V/Z/A → clipboard and undo operations
- Mouse click → `editable.hit_test()` to position cursor
- Mouse drag → extend selection
- Double-click → select word

### Rendering

Text input painting emits standard paint commands:

1. Background fill rect
2. Selection highlight rects (semi-transparent blue)
3. Text glyphs (via DrawGlyphs paint command)
4. Cursor rect (when focused and blink-on)

### New paint command

```rust
PaintCommand::DrawGlyphs {
    glyphs: Vec<PositionedGlyph>,
    color: Color,
}
```

`PositionedGlyph` contains `cache_key`, `x`, `y`, `width`, `height` — everything the renderer needs to look up the glyph atlas and emit textured quads.

### Cursor blinking

Handled via `spawn_after()` from the runtime, toggling a bool on the text input and requesting redraw. Blink resets on any keypress.

## Integration (`velox-app`)

### VeloxHandler changes

1. **Keyboard events** — handle `WindowEvent::KeyboardInput`: convert winit KeyEvent → velox KeyEvent, check ShortcutRegistry, dispatch to focused node
2. **Mouse events** — handle `CursorMoved`, `MouseInput`: track position, hit_test for focus, dispatch to target
3. **Clipboard** — own `arboard::Clipboard`, pass via EventContext
4. **Character input** — use winit `key_event.text` for printable characters

### New dependency

`arboard` for cross-platform clipboard access.

### Modified crates

- `velox-text` — gains EditableText, TextPosition, TextSelection, UndoStack, hit_test()
- `velox-scene` — gains EventHandler trait, KeyEvent/MouseEvent/EventContext, DrawGlyphs paint command, node event handler storage
- `velox-render` — handles DrawGlyphs command (rasterize → atlas → textured quads)
- `velox-app` — wires winit events → shortcut registry → focus → event handlers, owns clipboard

## Success Criteria

Phase 4 is done when:

1. Keyboard events route through shortcut registry → focused node
2. Mouse clicks update focus and position cursor in text fields
3. EditableText supports insert, delete, cursor movement, selection
4. Undo/redo works with coalescing (typing "hello" = one undo step)
5. Clipboard copy/cut/paste works via Cmd/Ctrl shortcuts
6. Text renders with visible cursor and selection highlighting
7. Demo app shows a functional text input field
