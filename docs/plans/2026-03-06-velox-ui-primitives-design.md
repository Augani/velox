# Velox UI Primitives Design

## Overview

Foundational UI element primitives for the Velox desktop framework — the base building blocks developers compose to build complex desktop applications. Zero-opinion primitives with full styling control, backed by a high-performance reconciler over Velox's retained scene graph.

## Core Architecture: Signal-Driven Reconciliation

Unlike immediate-mode frameworks that rebuild element trees every frame, Velox uses **signal-driven reconciliation** for Telegram Desktop-level performance:

1. A component's `render()` runs **once on mount**
2. Velox's reactive system (Signal/Computed) **tracks dependencies** during render
3. When a Signal changes, **only the affected component** re-renders
4. Re-render produces a new element subtree, reconciled against existing `NodeTree` nodes with **minimal patches**
5. Between signal changes: **zero work** — the retained graph handles everything

This is retained widgets with surgical updates — not per-frame rebuilds.

### Performance Targets

- < 4ms layout + paint per frame (240fps capable, 60fps minimum)
- Zero per-frame heap allocations in steady state (arena-allocated element trees)
- O(1) scroll through virtualized lists
- Only dirty subtrees re-layout/re-paint
- Paint-only style changes (bg color, opacity) skip layout entirely

## New Crate: `velox-ui`

```
crates/velox-ui/
  src/
    lib.rs
    element.rs          # Element, IntoElement, AnyElement
    styled.rs           # Styled trait, Style, StyleRefinement, Length
    interactive.rs      # InteractiveElement, event types
    parent.rs           # ParentElement trait
    component.rs        # Component trait, ViewContext
    reconciler.rs       # Element tree → NodeTree patching
    layout_engine.rs    # Taffy integration
    context.rs          # LayoutContext, PaintContext, EventContext
    elements/
      div.rs            # Universal container
      text.rs           # Text display
      img.rs            # Image display
      svg.rs            # Vector graphics / icons
      canvas.rs         # Custom paint commands
      list.rs           # Virtualized scrolling list
      input.rs          # Text input
      overlay.rs        # Positioned overlay/popup
```

**Dependencies:** `velox-scene`, `velox-style`, `velox-text`, `velox-reactive`, `velox-list`, `taffy`, `bumpalo`

## Core Traits

### Element — Lifecycle

```rust
pub trait Element: 'static {
    type State: 'static;

    fn layout(&mut self, state: &mut Self::State, cx: &mut LayoutContext) -> LayoutId;
    fn paint(&mut self, state: &mut Self::State, cx: &mut PaintContext);
}
```

Two-phase lifecycle (not three like GPUI) — Velox's retained graph handles hit-testing and accessibility, so prepaint is unnecessary.

### IntoElement — Conversion

```rust
pub trait IntoElement {
    type Element: Element;
    fn into_element(self) -> Self::Element;
}

// String literals work as elements directly
impl IntoElement for &str { /* wraps in text() */ }
impl IntoElement for String { /* wraps in text() */ }
```

### ParentElement — Child Composition

```rust
pub trait ParentElement: Sized {
    fn child(self, child: impl IntoElement) -> Self;
    fn children(self, children: impl IntoIterator<Item = impl IntoElement>) -> Self;
    fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self;
}
```

### Component — Stateful Views

```rust
pub trait Component: 'static + Sized {
    fn render(&self, cx: &mut ViewContext<Self>) -> impl IntoElement;
}
```

`ViewContext` provides access to reactive state (signals/computed), theme tokens, window/app context, and event emission.

## Styling System

### Design Principle: Zero-Opinion, Full Control

The framework provides mechanical capabilities (layout, color, borders, shadows, transforms). Developers compose them into whatever visual language they want. No built-in "button looks like a button."

### Three Layers

**1. `Style` struct** — flat data bag of every visual/layout property. No opinions, no defaults beyond "nothing rendered."

**2. `Styled` trait** — fluent builder API to set properties:

```rust
pub trait Styled: Sized {
    fn style_mut(&mut self) -> &mut Style;

    // Display
    fn flex(self) -> Self;
    fn flex_row(self) -> Self;
    fn flex_col(self) -> Self;
    fn grid(self) -> Self;
    fn block(self) -> Self;
    fn hidden(self) -> Self;

    // Sizing
    fn w(self, val: impl Into<Length>) -> Self;
    fn h(self, val: impl Into<Length>) -> Self;
    fn size(self, val: impl Into<Length>) -> Self;
    fn size_full(self) -> Self;
    fn min_w(self, val: impl Into<Length>) -> Self;
    fn max_w(self, val: impl Into<Length>) -> Self;
    fn min_h(self, val: impl Into<Length>) -> Self;
    fn max_h(self, val: impl Into<Length>) -> Self;
    fn aspect_ratio(self, ratio: f32) -> Self;

    // Spacing
    fn p(self, val: impl Into<Length>) -> Self;
    fn px(self, val: impl Into<Length>) -> Self;
    fn py(self, val: impl Into<Length>) -> Self;
    fn pt(self, val: impl Into<Length>) -> Self;
    fn pr(self, val: impl Into<Length>) -> Self;
    fn pb(self, val: impl Into<Length>) -> Self;
    fn pl(self, val: impl Into<Length>) -> Self;
    fn m(self, val: impl Into<Length>) -> Self;
    fn mx(self, val: impl Into<Length>) -> Self;
    fn my(self, val: impl Into<Length>) -> Self;
    fn mt(self, val: impl Into<Length>) -> Self;
    fn mr(self, val: impl Into<Length>) -> Self;
    fn mb(self, val: impl Into<Length>) -> Self;
    fn ml(self, val: impl Into<Length>) -> Self;
    fn gap(self, val: impl Into<Length>) -> Self;

    // Flex
    fn flex_1(self) -> Self;
    fn flex_grow(self) -> Self;
    fn flex_shrink(self) -> Self;
    fn flex_none(self) -> Self;
    fn flex_basis(self, val: impl Into<Length>) -> Self;
    fn flex_wrap(self) -> Self;
    fn items_start(self) -> Self;
    fn items_center(self) -> Self;
    fn items_end(self) -> Self;
    fn items_stretch(self) -> Self;
    fn justify_start(self) -> Self;
    fn justify_center(self) -> Self;
    fn justify_end(self) -> Self;
    fn justify_between(self) -> Self;
    fn justify_around(self) -> Self;
    fn justify_evenly(self) -> Self;
    fn self_start(self) -> Self;
    fn self_center(self) -> Self;
    fn self_end(self) -> Self;
    fn order(self, val: i32) -> Self;

    // Grid
    fn grid_cols(self, cols: Vec<TrackSize>) -> Self;
    fn grid_rows(self, rows: Vec<TrackSize>) -> Self;
    fn grid_cols_count(self, count: usize) -> Self;
    fn grid_col_span(self, span: usize) -> Self;
    fn grid_row_span(self, span: usize) -> Self;
    fn grid_area(self, name: &str) -> Self;
    fn grid_template(self, areas: Vec<&str>) -> Self;
    fn auto_flow_row(self) -> Self;
    fn auto_flow_col(self) -> Self;

    // Position
    fn relative(self) -> Self;
    fn absolute(self) -> Self;
    fn inset(self, val: impl Into<Length>) -> Self;
    fn top(self, val: impl Into<Length>) -> Self;
    fn right(self, val: impl Into<Length>) -> Self;
    fn bottom(self, val: impl Into<Length>) -> Self;
    fn left(self, val: impl Into<Length>) -> Self;
    fn z_index(self, val: i32) -> Self;

    // Borders
    fn border(self, width: impl Into<Length>) -> Self;
    fn border_t(self, width: impl Into<Length>) -> Self;
    fn border_r(self, width: impl Into<Length>) -> Self;
    fn border_b(self, width: impl Into<Length>) -> Self;
    fn border_l(self, width: impl Into<Length>) -> Self;
    fn border_color(self, color: impl Into<Color>) -> Self;
    fn border_dashed(self) -> Self;
    fn rounded(self, radius: impl Into<Length>) -> Self;
    fn rounded_t(self, radius: impl Into<Length>) -> Self;
    fn rounded_b(self, radius: impl Into<Length>) -> Self;
    fn rounded_full(self) -> Self;

    // Background
    fn bg(self, color: impl Into<Color>) -> Self;
    fn bg_gradient(self, gradient: Gradient) -> Self;

    // Shadows
    fn shadow(self, shadow: BoxShadow) -> Self;
    fn shadow_sm(self) -> Self;
    fn shadow_md(self) -> Self;
    fn shadow_lg(self) -> Self;
    fn shadow_inner(self, shadow: BoxShadow) -> Self;

    // Text
    fn text_color(self, color: impl Into<Color>) -> Self;
    fn text_size(self, size: impl Into<Length>) -> Self;
    fn text_xs(self) -> Self;
    fn text_sm(self) -> Self;
    fn text_base(self) -> Self;
    fn text_lg(self) -> Self;
    fn text_xl(self) -> Self;
    fn font_weight(self, weight: FontWeight) -> Self;
    fn font_family(self, family: &str) -> Self;
    fn line_height(self, val: f32) -> Self;
    fn letter_spacing(self, val: f32) -> Self;
    fn text_align(self, align: TextAlign) -> Self;
    fn text_ellipsis(self) -> Self;
    fn text_wrap(self) -> Self;
    fn text_nowrap(self) -> Self;
    fn text_decoration(self, decoration: TextDecoration) -> Self;

    // Effects
    fn opacity(self, val: f32) -> Self;
    fn blend_mode(self, mode: BlendMode) -> Self;
    fn backdrop_blur(self, radius: f32) -> Self;
    fn transform(self, transform: Transform) -> Self;

    // Overflow
    fn overflow_hidden(self) -> Self;
    fn overflow_scroll(self) -> Self;
    fn overflow_x_hidden(self) -> Self;
    fn overflow_y_scroll(self) -> Self;

    // Cursor
    fn cursor_pointer(self) -> Self;
    fn cursor_text(self) -> Self;
    fn cursor_grab(self) -> Self;
    fn cursor_not_allowed(self) -> Self;

    // Transitions (ties into velox-animation)
    fn transition(self, property: &str, duration: Duration, easing: Easing) -> Self;
}
```

**3. `StyleRefinement`** — partial style overlay for composable presets:

```rust
pub struct StyleRefinement { /* all fields Option<T> */ }

impl StyleRefinement {
    pub fn new() -> Self;
    pub fn merge(self, other: StyleRefinement) -> Self;
}

// Developers create reusable style presets
fn card_style() -> StyleRefinement {
    StyleRefinement::new()
        .bg(colors::surface_1)
        .rounded(px(12))
        .shadow(shadow::md())
        .p(px(16))
}
```

### Conditional Styles

Full Rust logic, no CSS pseudoclasses:

```rust
div()
    .bg(theme.surface)
    .hover(|s| s.bg(theme.surface_hover))
    .active(|s| s.bg(theme.surface_active).scale(0.98))
    .when(is_selected, |s| s.border(px(2)).border_color(theme.accent))
    .when(is_disabled, |s| s.opacity(0.5).cursor_not_allowed())
```

### Length Units

```rust
px(16.0)          // exact pixels (logical, DPI-scaled)
pct(50.0)         // percentage of parent
fr(1.0)           // fractional unit (grid)
auto()            // content-sized
Length::ZERO      // 0
```

## Layout Modes

Powered by Taffy (implements CSS flexbox and grid specs). Three modes:

### Flex (Default)

One-dimensional flow:

```rust
// Horizontal
div().flex_row().items_center().gap(px(8))
    .child(icon).child(text("Title").flex_1()).child(button)

// Vertical
div().flex_col().gap(px(16))
    .child(header()).child(content().flex_1()).child(footer())

// Wrapping
div().flex_row().flex_wrap().gap(px(4))
    .children(tags.iter().map(|t| tag_chip(t)))
```

### Grid

Two-dimensional layouts:

```rust
// Explicit columns
div().grid().grid_cols(vec![fr(1), fr(2), fr(1)]).gap(px(16))
    .child(sidebar()).child(main()).child(detail())

// Equal-width shorthand
div().grid().grid_cols_count(3).gap(px(12))
    .children(items.iter().map(|item| card(item)))

// Named areas
div().grid().grid_template(vec![
    "header  header  header",
    "sidebar content detail",
    "footer  footer  footer",
])
```

### Absolute Positioning

Relative to nearest positioned ancestor:

```rust
div().relative()
    .child(avatar(user))
    .child(
        div().absolute().bottom(px(0)).right(px(0))
            .size(px(12)).rounded_full().bg(green())
    )
```

## Base Elements

Eight primitives. Everything developers build composes from these:

### `div()` — Universal Container

```rust
div()
    .flex_row().gap(px(8)).p(px(16)).bg(theme.surface)
    .child(/* anything */)
```

Layout (flex/grid), styling, events, children. Maps to a `NodeTree` node with Taffy layout.

### `text(content)` — Text Display

```rust
text("Hello world")
    .text_lg().font_weight(FontWeight::BOLD)
    .text_color(theme.foreground).text_ellipsis()
```

Wraps cosmic-text via velox-text. Supports wrapping, truncation, selection (opt-in), rich text spans via `text_runs()`.

### `img(source)` — Image Display

```rust
img(ImageSource::path("avatar.png"))
    .size(px(48)).rounded_full()
    .object_fit(ObjectFit::Cover)
    .fallback(|| div().bg(gray(200)))
```

Async decode on compute pool, automatic lifecycle. Wraps velox-media ImageHandle.

### `svg(data)` — Vector Graphics / Icons

```rust
svg(include_str!("icons/chevron.svg"))
    .size(px(16)).text_color(theme.icon)
```

Fill color follows `text_color`.

### `canvas(callback)` — Custom Paint Commands

```rust
canvas(|bounds, cx| {
    cx.paint.fill_rect(bounds, Color::RED);
    cx.paint.stroke_rect(bounds.inset(px(2)), Color::BLACK, 1.0);
})
.w(px(200)).h(px(150))
```

Escape hatch for charts, custom visualizations. Direct `CommandList` access.

### `list(state, render_item)` — Virtualized List

```rust
let list_state = ListState::new(count, ListAlignment::Bottom, px(44.0));

list(list_state, |index, cx| {
    message_bubble(&messages[index])
})
.flex_1()
```

Only renders visible items + buffer. O(1) scroll. Variable heights. Scroll anchoring. Backed by velox-list.

### `input()` — Text Input

```rust
input()
    .placeholder("Type a message...")
    .value(signal.get(cx))
    .on_change(|val, cx| signal.set(cx, val))
    .on_submit(|val, cx| cx.emit(SendMessage(val)))
    .multiline().max_lines(5)
```

Cursor, selection, undo/redo, IME composition, clipboard. Wraps velox-text EditableText.

### `overlay()` — Positioned Overlay

```rust
overlay()
    .anchor(button_bounds)
    .placement(Placement::Below)
    .offset(px(4))
    .child(dropdown_menu())
```

Z-ordering, backdrop dismiss, focus trapping (modal). Backed by velox-scene OverlayStack.

## InteractiveElement Trait

```rust
pub trait InteractiveElement: Sized {
    fn on_mouse_down(self, handler: impl Fn(&MouseDownEvent, &mut EventContext) + 'static) -> Self;
    fn on_mouse_up(self, handler: impl Fn(&MouseUpEvent, &mut EventContext) + 'static) -> Self;
    fn on_click(self, handler: impl Fn(&ClickEvent, &mut EventContext) + 'static) -> Self;
    fn on_hover(self, handler: impl Fn(&bool, &mut EventContext) + 'static) -> Self;
    fn on_scroll(self, handler: impl Fn(&ScrollEvent, &mut EventContext) + 'static) -> Self;
    fn on_key_down(self, handler: impl Fn(&KeyDownEvent, &mut EventContext) + 'static) -> Self;
    fn on_key_up(self, handler: impl Fn(&KeyUpEvent, &mut EventContext) + 'static) -> Self;
    fn on_focus(self, handler: impl Fn(&FocusEvent, &mut EventContext) + 'static) -> Self;
    fn on_blur(self, handler: impl Fn(&BlurEvent, &mut EventContext) + 'static) -> Self;
    fn focusable(self) -> Self;
    fn draggable(self, payload: impl Into<DragPayload>) -> Self;
    fn on_drop(self, handler: impl Fn(&DropEvent, &mut EventContext) + 'static) -> Self;
}
```

## Reconciler

### ReconcilerSlot — Retained Mapping

Each component stores a slot tree from its last render:

```rust
struct ReconcilerSlot {
    node_id: NodeId,
    taffy_node: TaffyNodeId,
    element_type: TypeId,
    key: Option<ElementKey>,
    children: Vec<ReconcilerSlot>,
}
```

### Three Modes

1. **Mount** — first render. Walk element tree, create NodeTree nodes 1:1, register Taffy layout nodes.
2. **Patch** — signal-driven update. Diff new element subtree against existing slots, emit minimal patches.
3. **Unmount** — component removed. Walk nodes, remove from NodeTree, deregister Taffy nodes, drop handlers.

### Diffing Rules

- Same position + same type → update in place (diff style, text, handlers)
- Same position + different type → destroy old, create new
- Keyed children → match by key, detect insertions/removals/moves in O(n)
- Unkeyed children → match by index

### Patch Types

```rust
enum Patch {
    CreateNode { parent: NodeId, index: usize, element: AnyElement },
    RemoveNode { node_id: NodeId },
    MoveNode { node_id: NodeId, new_parent: NodeId, new_index: usize },
    UpdateStyle { node_id: NodeId, style: StyleRefinement },
    UpdateText { node_id: NodeId, content: SharedString },
    UpdateImage { node_id: NodeId, source: ImageSource },
    SetEventHandler { node_id: NodeId, event: EventType, handler: AnyHandler },
    RemoveEventHandler { node_id: NodeId, event: EventType },
    UpdateTaffyStyle { taffy_node: TaffyNodeId, style: taffy::Style },
    MarkDirty { node_id: NodeId },
}
```

### Style Diffing — Layout vs Paint

Style changes split into two categories:

- **Layout-affecting** (triggers Taffy recompute): width, height, padding, margin, flex, grid, display, position, gap, min/max
- **Paint-only** (skip layout): background, border color, shadow, opacity, text color, cursor

A hover that only changes `bg` → zero layout work, only repaint.

### Batched Updates

Multiple signal changes in one frame:

1. Collect all dirty components
2. Sort by tree depth (parents first — parent render may make child redundant)
3. Render each once
4. Collect all patches, apply in one batch to NodeTree
5. One Taffy layout pass for all dirty subtrees
6. One paint pass

### Arena Allocation

Element trees allocated in `bumpalo::Bump`, reset after reconciliation. Zero heap allocations in steady state.

## Integration with Existing Velox

The reconciler patches `NodeTree`, so all downstream systems work unchanged:

```
Component::render()
  → Element tree (arena allocated)
  → Reconciler diffs against ReconcilerSlots
  → Patches applied to NodeTree
  → Taffy computes layout (cached, incremental)
  → Existing systems:
      ├── Hit testing (NodeTree traversal)
      ├── Focus management (FocusState)
      ├── Accessibility (AccessibilityTreeSnapshot)
      ├── Overlays (OverlayStack)
      ├── Drag/drop (DragState)
      ├── Paint commands → Renderer → GPU
```

## Performance Summary

| Scenario | Work Done |
|----------|-----------|
| Nothing changed | Zero — retained graph handles frame |
| Hover effect (bg color) | 1 style diff, 1 paint patch, no layout |
| Text update | 1 text diff, 1 paint patch, layout only if size changed |
| New message in chat | 1 keyed insert, 1 node created, partial layout |
| Scroll 100k messages | 0 reconciliation — virtual list swaps visibility flags |
| Window resize | 1 Taffy layout pass (cached subtrees), 1 full paint |
| Theme switch | Batch style patches for visible nodes, 1 paint pass |

## Component Model Example

```rust
struct ChatView {
    messages: Signal<Vec<Message>>,
    input_text: Signal<String>,
    list_state: ListState,
}

impl Component for ChatView {
    fn render(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let messages = self.messages.get(cx);
        let input_text = self.input_text.get(cx);

        div().flex_col().size_full()
            .child(
                list(self.list_state.clone(), |index, cx| {
                    message_bubble(&messages[index], cx)
                }).flex_1()
            )
            .child(
                div().flex_row().p(px(8)).gap(px(8))
                    .border_t(px(1)).border_color(cx.theme().border)
                    .child(
                        input().flex_1()
                            .placeholder("Message...")
                            .value(input_text)
                            .on_submit(|text, cx| cx.emit(SendMessage(text)))
                    )
                    .child(
                        div().cursor_pointer().p(px(8)).rounded(px(8))
                            .bg(cx.theme().accent)
                            .hover(|s| s.bg(cx.theme().accent_hover))
                            .on_click(|_, cx| cx.emit(SendClicked))
                            .child(svg(icons::SEND).size(px(20)))
                    )
            )
    }
}
```

## Workspace Changes

### New dependency
```toml
taffy = "0.7"
bumpalo = "3"
```

### New crate member
```toml
"crates/velox-ui"
```

### Facade re-exports
Add `velox-ui` to `velox/src/lib.rs` prelude: `div`, `text`, `img`, `svg`, `canvas`, `list`, `input`, `overlay`, `Component`, `Element`, `Styled`, `InteractiveElement`, `ParentElement`, `IntoElement`, `px`, `pct`, `fr`, `auto`.
