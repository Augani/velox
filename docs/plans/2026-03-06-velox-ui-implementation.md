# Velox UI Primitives Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the `velox-ui` crate — 8 base elements, core traits, Taffy layout engine, and signal-driven reconciler over Velox's retained scene graph.

**Architecture:** GPUI-inspired element builders (`div()`, `text()`, etc.) with fluent styling, backed by Taffy for CSS flexbox/grid layout. Elements are declarative descriptions; a reconciler converts them to `NodeTree` nodes with minimal patches on re-render. Reactive signals trigger scoped re-renders, not per-frame rebuilds.

**Tech Stack:** Rust (edition 2024), taffy 0.7 (flexbox/grid), bumpalo 3 (arena allocation), velox-scene (NodeTree), velox-reactive (Signal), velox-style (Theme)

---

## Existing Types Reference

Before you start, know these existing types you'll depend on:

- **`NodeTree`** (`velox-scene/src/tree.rs`): SlotMap-based retained tree. Methods: `insert(parent)→NodeId`, `remove(id)`, `set_rect(id, Rect)`, `set_visible(id, bool)`, `set_painter(id, impl Painter)`, `set_layout(id, impl Layout)`, `set_event_handler(id, impl EventHandler)`, `run_layout()`, `run_paint(&mut CommandList)`, `mark_layout_dirty(id)`, `mark_paint_dirty(id)`
- **`NodeId`** (`velox-scene/src/node.rs`): slotmap key type
- **`Rect`** (`velox-scene/src/geometry.rs`): `{ x: f32, y: f32, width: f32, height: f32 }` with `new()`, `zero()`, `contains(Point)`
- **`Point`**: `{ x: f32, y: f32 }`
- **`Size`**: `{ width: f32, height: f32 }`
- **`Color`** (`velox-scene/src/paint.rs`): `{ r: u8, g: u8, b: u8, a: u8 }` with `rgb()`, `rgba()`
- **`PaintCommand`**: FillRect, StrokeRect, DrawGlyphs, DrawImage, PushClip/PopClip, PushLayer/PopLayer, BoxShadow
- **`CommandList`**: `fill_rect()`, `stroke_rect()`, `push_clip()`, `pop_clip()`, `push_layer()`, `pop_layer()`, `box_shadow()`
- **`BlendMode`**: Normal, Multiply, Screen, Overlay
- **`Painter` trait**: `fn paint(&self, rect: Rect, commands: &mut CommandList)`
- **`Layout` trait**: `fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree)`
- **`EventHandler` trait**: `handle_key()`, `handle_mouse()`, `handle_scroll()`, `handle_ime()`, `handle_focus()`
- **`Signal<T>`** (`velox-reactive`): `new(value)`, `get()`, `set(value)`, `subscribe(callback)→Subscription`
- **`Theme`** (`velox-style`): `{ name, palette: Palette, space: SpaceScale, radius: RadiusScale, typography: TypographyTokens }`
- **`MouseEvent`**: `{ position: Point, button: MouseButton, state: ButtonState, click_count: u32, modifiers }`
- **`KeyEvent`**: `{ key: Key, modifiers, state: KeyState, text: Option<String> }`
- **`ScrollEvent`**: `{ delta_x, delta_y, modifiers }`

---

## Task 1: Crate Scaffold + Length Units

**Files:**
- Create: `crates/velox-ui/Cargo.toml`
- Create: `crates/velox-ui/src/lib.rs`
- Create: `crates/velox-ui/src/length.rs`
- Modify: `Cargo.toml` (workspace root)

**Step 1: Add workspace dependencies and member**

In workspace root `Cargo.toml`, add to `[workspace.dependencies]`:
```toml
velox-ui = { path = "crates/velox-ui" }
taffy = "0.7"
bumpalo = "3"
```

Add `"crates/velox-ui"` to the `members` array.

**Step 2: Create crate Cargo.toml**

```toml
[package]
name = "velox-ui"
version.workspace = true
edition.workspace = true

[dependencies]
velox-scene = { workspace = true }
velox-reactive = { workspace = true }
velox-style = { workspace = true }
taffy = { workspace = true }

[dev-dependencies]
```

**Step 3: Write tests for Length**

In `crates/velox-ui/src/length.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Px(f32),
    Pct(f32),
    Fr(f32),
    Auto,
}

impl Length {
    pub const ZERO: Self = Self::Px(0.0);
}

pub fn px(val: f32) -> Length {
    Length::Px(val)
}

pub fn pct(val: f32) -> Length {
    Length::Pct(val)
}

pub fn fr(val: f32) -> Length {
    Length::Fr(val)
}

pub fn auto() -> Length {
    Length::Auto
}

impl From<f32> for Length {
    fn from(val: f32) -> Self {
        Length::Px(val)
    }
}

impl From<i32> for Length {
    fn from(val: i32) -> Self {
        Length::Px(val as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_constructor() {
        assert_eq!(px(16.0), Length::Px(16.0));
    }

    #[test]
    fn pct_constructor() {
        assert_eq!(pct(50.0), Length::Pct(50.0));
    }

    #[test]
    fn fr_constructor() {
        assert_eq!(fr(1.0), Length::Fr(1.0));
    }

    #[test]
    fn auto_constructor() {
        assert_eq!(auto(), Length::Auto);
    }

    #[test]
    fn zero_constant() {
        assert_eq!(Length::ZERO, Length::Px(0.0));
    }

    #[test]
    fn from_f32() {
        let len: Length = 10.0_f32.into();
        assert_eq!(len, Length::Px(10.0));
    }

    #[test]
    fn from_i32() {
        let len: Length = 10_i32.into();
        assert_eq!(len, Length::Px(10.0));
    }
}
```

**Step 4: Create lib.rs**

```rust
mod length;

pub use length::{auto, fr, px, pct, Length};
```

**Step 5: Verify build**

Run: `cargo test -p velox-ui`
Expected: 7 tests pass

**Step 6: Commit**

```bash
git add crates/velox-ui/ Cargo.toml
git commit -m "feat(velox-ui): scaffold crate with Length units"
```

---

## Task 2: Style Struct

**Files:**
- Create: `crates/velox-ui/src/style.rs`
- Modify: `crates/velox-ui/src/lib.rs`

The Style struct holds every visual and layout property. All values are `Option<T>` so they can be selectively set.

**Step 1: Write the Style struct**

```rust
use velox_scene::Color;
use crate::length::Length;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Flex,
    Grid,
    Block,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    Default,
    Pointer,
    Text,
    Grab,
    Grabbing,
    NotAllowed,
    Move,
    Crosshair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    Light,
    Normal,
    Medium,
    Semibold,
    Bold,
    Extrabold,
    Black,
}

impl FontWeight {
    pub fn to_u16(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::Light => 300,
            Self::Normal => 400,
            Self::Medium => 500,
            Self::Semibold => 600,
            Self::Bold => 700,
            Self::Extrabold => 800,
            Self::Black => 900,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflow {
    Wrap,
    NoWrap,
    Ellipsis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecoration {
    None,
    Underline,
    LineThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Solid,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadowStyle {
    pub color: Color,
    pub blur_radius: f32,
    pub spread: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub inset: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFit {
    Contain,
    Cover,
    Fill,
    None,
    ScaleDown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridTemplate {
    pub columns: Vec<TrackSize>,
    pub rows: Vec<TrackSize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackSize {
    Px(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Style {
    pub display: Option<Display>,
    pub position: Option<Position>,

    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_width: Option<Length>,
    pub min_height: Option<Length>,
    pub max_width: Option<Length>,
    pub max_height: Option<Length>,
    pub aspect_ratio: Option<f32>,

    pub padding_top: Option<Length>,
    pub padding_right: Option<Length>,
    pub padding_bottom: Option<Length>,
    pub padding_left: Option<Length>,

    pub margin_top: Option<Length>,
    pub margin_right: Option<Length>,
    pub margin_bottom: Option<Length>,
    pub margin_left: Option<Length>,

    pub inset_top: Option<Length>,
    pub inset_right: Option<Length>,
    pub inset_bottom: Option<Length>,
    pub inset_left: Option<Length>,

    pub flex_direction: Option<FlexDirection>,
    pub flex_wrap: Option<FlexWrap>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<Length>,
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignItems>,
    pub justify_content: Option<JustifyContent>,
    pub gap: Option<Length>,
    pub row_gap: Option<Length>,
    pub column_gap: Option<Length>,
    pub order: Option<i32>,

    pub grid_template: Option<GridTemplate>,
    pub grid_column_start: Option<i16>,
    pub grid_column_end: Option<i16>,
    pub grid_row_start: Option<i16>,
    pub grid_row_end: Option<i16>,

    pub overflow_x: Option<Overflow>,
    pub overflow_y: Option<Overflow>,

    pub background: Option<Color>,
    pub border_top_width: Option<f32>,
    pub border_right_width: Option<f32>,
    pub border_bottom_width: Option<f32>,
    pub border_left_width: Option<f32>,
    pub border_color: Option<Color>,
    pub border_style: Option<BorderStyle>,
    pub border_radius_tl: Option<f32>,
    pub border_radius_tr: Option<f32>,
    pub border_radius_bl: Option<f32>,
    pub border_radius_br: Option<f32>,

    pub box_shadows: Vec<BoxShadowStyle>,

    pub opacity: Option<f32>,
    pub z_index: Option<i32>,
    pub cursor: Option<CursorStyle>,

    pub text_color: Option<Color>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_family: Option<String>,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub text_align: Option<TextAlign>,
    pub text_overflow: Option<TextOverflow>,
    pub text_decoration: Option<TextDecoration>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: &Style) {
        macro_rules! merge_field {
            ($field:ident) => {
                if other.$field.is_some() {
                    self.$field = other.$field.clone();
                }
            };
        }

        merge_field!(display);
        merge_field!(position);
        merge_field!(width);
        merge_field!(height);
        merge_field!(min_width);
        merge_field!(min_height);
        merge_field!(max_width);
        merge_field!(max_height);
        merge_field!(aspect_ratio);
        merge_field!(padding_top);
        merge_field!(padding_right);
        merge_field!(padding_bottom);
        merge_field!(padding_left);
        merge_field!(margin_top);
        merge_field!(margin_right);
        merge_field!(margin_bottom);
        merge_field!(margin_left);
        merge_field!(inset_top);
        merge_field!(inset_right);
        merge_field!(inset_bottom);
        merge_field!(inset_left);
        merge_field!(flex_direction);
        merge_field!(flex_wrap);
        merge_field!(flex_grow);
        merge_field!(flex_shrink);
        merge_field!(flex_basis);
        merge_field!(align_items);
        merge_field!(align_self);
        merge_field!(justify_content);
        merge_field!(gap);
        merge_field!(row_gap);
        merge_field!(column_gap);
        merge_field!(order);
        merge_field!(grid_template);
        merge_field!(grid_column_start);
        merge_field!(grid_column_end);
        merge_field!(grid_row_start);
        merge_field!(grid_row_end);
        merge_field!(overflow_x);
        merge_field!(overflow_y);
        merge_field!(background);
        merge_field!(border_top_width);
        merge_field!(border_right_width);
        merge_field!(border_bottom_width);
        merge_field!(border_left_width);
        merge_field!(border_color);
        merge_field!(border_style);
        merge_field!(border_radius_tl);
        merge_field!(border_radius_tr);
        merge_field!(border_radius_bl);
        merge_field!(border_radius_br);
        merge_field!(opacity);
        merge_field!(z_index);
        merge_field!(cursor);
        merge_field!(text_color);
        merge_field!(font_size);
        merge_field!(font_weight);
        merge_field!(font_family);
        merge_field!(line_height);
        merge_field!(letter_spacing);
        merge_field!(text_align);
        merge_field!(text_overflow);
        merge_field!(text_decoration);

        if !other.box_shadows.is_empty() {
            self.box_shadows = other.box_shadows.clone();
        }
    }

    pub fn is_layout_affecting_different(&self, other: &Style) -> bool {
        self.display != other.display
            || self.position != other.position
            || self.width != other.width
            || self.height != other.height
            || self.min_width != other.min_width
            || self.min_height != other.min_height
            || self.max_width != other.max_width
            || self.max_height != other.max_height
            || self.aspect_ratio != other.aspect_ratio
            || self.padding_top != other.padding_top
            || self.padding_right != other.padding_right
            || self.padding_bottom != other.padding_bottom
            || self.padding_left != other.padding_left
            || self.margin_top != other.margin_top
            || self.margin_right != other.margin_right
            || self.margin_bottom != other.margin_bottom
            || self.margin_left != other.margin_left
            || self.inset_top != other.inset_top
            || self.inset_right != other.inset_right
            || self.inset_bottom != other.inset_bottom
            || self.inset_left != other.inset_left
            || self.flex_direction != other.flex_direction
            || self.flex_wrap != other.flex_wrap
            || self.flex_grow != other.flex_grow
            || self.flex_shrink != other.flex_shrink
            || self.flex_basis != other.flex_basis
            || self.align_items != other.align_items
            || self.align_self != other.align_self
            || self.justify_content != other.justify_content
            || self.gap != other.gap
            || self.row_gap != other.row_gap
            || self.column_gap != other.column_gap
            || self.grid_template != other.grid_template
            || self.grid_column_start != other.grid_column_start
            || self.grid_column_end != other.grid_column_end
            || self.grid_row_start != other.grid_row_start
            || self.grid_row_end != other.grid_row_end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style_all_none() {
        let style = Style::new();
        assert!(style.display.is_none());
        assert!(style.width.is_none());
        assert!(style.background.is_none());
        assert!(style.box_shadows.is_empty());
    }

    #[test]
    fn merge_overwrites_set_fields() {
        let mut base = Style::new();
        base.display = Some(Display::Flex);
        base.width = Some(Length::Px(100.0));

        let overlay = Style {
            width: Some(Length::Px(200.0)),
            background: Some(Color::rgb(255, 0, 0)),
            ..Style::default()
        };

        base.merge(&overlay);
        assert_eq!(base.display, Some(Display::Flex));
        assert_eq!(base.width, Some(Length::Px(200.0)));
        assert_eq!(base.background, Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn merge_preserves_unset_fields() {
        let mut base = Style::new();
        base.opacity = Some(0.5);

        let overlay = Style::default();
        base.merge(&overlay);
        assert_eq!(base.opacity, Some(0.5));
    }

    #[test]
    fn layout_diff_detects_width_change() {
        let a = Style {
            width: Some(Length::Px(100.0)),
            ..Style::default()
        };
        let b = Style {
            width: Some(Length::Px(200.0)),
            ..Style::default()
        };
        assert!(a.is_layout_affecting_different(&b));
    }

    #[test]
    fn layout_diff_ignores_color_change() {
        let a = Style {
            background: Some(Color::rgb(255, 0, 0)),
            ..Style::default()
        };
        let b = Style {
            background: Some(Color::rgb(0, 255, 0)),
            ..Style::default()
        };
        assert!(!a.is_layout_affecting_different(&b));
    }

    #[test]
    fn font_weight_to_u16() {
        assert_eq!(FontWeight::Normal.to_u16(), 400);
        assert_eq!(FontWeight::Bold.to_u16(), 700);
    }
}
```

**Step 2: Add to lib.rs**

```rust
mod length;
mod style;

pub use length::{auto, fr, px, pct, Length};
pub use style::*;
```

**Step 3: Verify**

Run: `cargo test -p velox-ui`
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(velox-ui): Style struct with all visual/layout properties"
```

---

## Task 3: Styled Trait (Fluent Builder API)

**Files:**
- Create: `crates/velox-ui/src/styled.rs`
- Modify: `crates/velox-ui/src/lib.rs`

This is the trait that enables `.flex_row().gap(px(8)).bg(red())` fluent API on any element type.

**Step 1: Write the Styled trait**

```rust
use velox_scene::Color;
use crate::length::Length;
use crate::style::*;

pub trait Styled: Sized {
    fn style_mut(&mut self) -> &mut Style;

    fn display(mut self, val: Display) -> Self {
        self.style_mut().display = Some(val);
        self
    }

    fn flex(self) -> Self { self.display(Display::Flex) }
    fn flex_row(mut self) -> Self {
        self.style_mut().display = Some(Display::Flex);
        self.style_mut().flex_direction = Some(FlexDirection::Row);
        self
    }
    fn flex_col(mut self) -> Self {
        self.style_mut().display = Some(Display::Flex);
        self.style_mut().flex_direction = Some(FlexDirection::Column);
        self
    }
    fn grid(self) -> Self { self.display(Display::Grid) }
    fn block(self) -> Self { self.display(Display::Block) }
    fn hidden(self) -> Self { self.display(Display::None) }

    fn w(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().width = Some(val.into()); self
    }
    fn h(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().height = Some(val.into()); self
    }
    fn size(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().width = Some(v);
        self.style_mut().height = Some(v);
        self
    }
    fn size_full(self) -> Self {
        self.w(pct(100.0)).h(pct(100.0))
    }
    fn min_w(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().min_width = Some(val.into()); self
    }
    fn max_w(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().max_width = Some(val.into()); self
    }
    fn min_h(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().min_height = Some(val.into()); self
    }
    fn max_h(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().max_height = Some(val.into()); self
    }
    fn w_full(self) -> Self { self.w(pct(100.0)) }
    fn h_full(self) -> Self { self.h(pct(100.0)) }
    fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.style_mut().aspect_ratio = Some(ratio); self
    }

    fn p(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        let s = self.style_mut();
        s.padding_top = Some(v); s.padding_right = Some(v);
        s.padding_bottom = Some(v); s.padding_left = Some(v);
        self
    }
    fn px_pad(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().padding_left = Some(v);
        self.style_mut().padding_right = Some(v);
        self
    }
    fn py(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().padding_top = Some(v);
        self.style_mut().padding_bottom = Some(v);
        self
    }
    fn pt(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_top = Some(val.into()); self
    }
    fn pr(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_right = Some(val.into()); self
    }
    fn pb(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_bottom = Some(val.into()); self
    }
    fn pl(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().padding_left = Some(val.into()); self
    }

    fn m(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        let s = self.style_mut();
        s.margin_top = Some(v); s.margin_right = Some(v);
        s.margin_bottom = Some(v); s.margin_left = Some(v);
        self
    }
    fn mx(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().margin_left = Some(v);
        self.style_mut().margin_right = Some(v);
        self
    }
    fn my(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        self.style_mut().margin_top = Some(v);
        self.style_mut().margin_bottom = Some(v);
        self
    }
    fn mt(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_top = Some(val.into()); self
    }
    fn mr(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_right = Some(val.into()); self
    }
    fn mb(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_bottom = Some(val.into()); self
    }
    fn ml(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().margin_left = Some(val.into()); self
    }

    fn gap(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().gap = Some(val.into()); self
    }
    fn row_gap(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().row_gap = Some(val.into()); self
    }
    fn column_gap(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().column_gap = Some(val.into()); self
    }

    fn flex_1(mut self) -> Self {
        self.style_mut().flex_grow = Some(1.0);
        self.style_mut().flex_shrink = Some(1.0);
        self.style_mut().flex_basis = Some(Length::Px(0.0));
        self
    }
    fn flex_grow(mut self) -> Self {
        self.style_mut().flex_grow = Some(1.0); self
    }
    fn flex_shrink(mut self) -> Self {
        self.style_mut().flex_shrink = Some(1.0); self
    }
    fn flex_none(mut self) -> Self {
        self.style_mut().flex_grow = Some(0.0);
        self.style_mut().flex_shrink = Some(0.0);
        self
    }
    fn flex_basis(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().flex_basis = Some(val.into()); self
    }
    fn flex_wrap(mut self) -> Self {
        self.style_mut().flex_wrap = Some(FlexWrap::Wrap); self
    }

    fn items_start(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::Start); self
    }
    fn items_center(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::Center); self
    }
    fn items_end(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::End); self
    }
    fn items_stretch(mut self) -> Self {
        self.style_mut().align_items = Some(AlignItems::Stretch); self
    }

    fn justify_start(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::Start); self
    }
    fn justify_center(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::Center); self
    }
    fn justify_end(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::End); self
    }
    fn justify_between(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::SpaceBetween); self
    }
    fn justify_around(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::SpaceAround); self
    }
    fn justify_evenly(mut self) -> Self {
        self.style_mut().justify_content = Some(JustifyContent::SpaceEvenly); self
    }

    fn self_start(mut self) -> Self {
        self.style_mut().align_self = Some(AlignItems::Start); self
    }
    fn self_center(mut self) -> Self {
        self.style_mut().align_self = Some(AlignItems::Center); self
    }
    fn self_end(mut self) -> Self {
        self.style_mut().align_self = Some(AlignItems::End); self
    }
    fn order(mut self, val: i32) -> Self {
        self.style_mut().order = Some(val); self
    }

    fn grid_cols(mut self, cols: Vec<TrackSize>) -> Self {
        let s = self.style_mut();
        let tpl = s.grid_template.get_or_insert_with(|| GridTemplate { columns: vec![], rows: vec![] });
        tpl.columns = cols;
        self
    }
    fn grid_rows(mut self, rows: Vec<TrackSize>) -> Self {
        let s = self.style_mut();
        let tpl = s.grid_template.get_or_insert_with(|| GridTemplate { columns: vec![], rows: vec![] });
        tpl.rows = rows;
        self
    }
    fn grid_cols_count(self, count: usize) -> Self {
        self.grid_cols(vec![TrackSize::Fr(1.0); count])
    }
    fn grid_col_span(mut self, span: usize) -> Self {
        self.style_mut().grid_column_end = Some(span as i16 + 1);
        self.style_mut().grid_column_start = Some(1);
        self
    }
    fn grid_row_span(mut self, span: usize) -> Self {
        self.style_mut().grid_row_end = Some(span as i16 + 1);
        self.style_mut().grid_row_start = Some(1);
        self
    }

    fn relative(mut self) -> Self {
        self.style_mut().position = Some(Position::Relative); self
    }
    fn absolute(mut self) -> Self {
        self.style_mut().position = Some(Position::Absolute); self
    }
    fn inset(mut self, val: impl Into<Length>) -> Self {
        let v = val.into();
        let s = self.style_mut();
        s.inset_top = Some(v); s.inset_right = Some(v);
        s.inset_bottom = Some(v); s.inset_left = Some(v);
        self
    }
    fn top(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_top = Some(val.into()); self
    }
    fn right(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_right = Some(val.into()); self
    }
    fn bottom(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_bottom = Some(val.into()); self
    }
    fn left(mut self, val: impl Into<Length>) -> Self {
        self.style_mut().inset_left = Some(val.into()); self
    }
    fn z_index(mut self, val: i32) -> Self {
        self.style_mut().z_index = Some(val); self
    }

    fn border(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else { return self };
        let s = self.style_mut();
        s.border_top_width = Some(w); s.border_right_width = Some(w);
        s.border_bottom_width = Some(w); s.border_left_width = Some(w);
        self
    }
    fn border_t(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else { return self };
        self.style_mut().border_top_width = Some(w); self
    }
    fn border_r(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else { return self };
        self.style_mut().border_right_width = Some(w); self
    }
    fn border_b(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else { return self };
        self.style_mut().border_bottom_width = Some(w); self
    }
    fn border_l(mut self, width: impl Into<Length>) -> Self {
        let Length::Px(w) = width.into() else { return self };
        self.style_mut().border_left_width = Some(w); self
    }
    fn border_color(mut self, color: impl Into<Color>) -> Self {
        self.style_mut().border_color = Some(color.into()); self
    }
    fn border_dashed(mut self) -> Self {
        self.style_mut().border_style = Some(BorderStyle::Dashed); self
    }
    fn rounded(mut self, radius: impl Into<Length>) -> Self {
        let Length::Px(r) = radius.into() else { return self };
        let s = self.style_mut();
        s.border_radius_tl = Some(r); s.border_radius_tr = Some(r);
        s.border_radius_bl = Some(r); s.border_radius_br = Some(r);
        self
    }
    fn rounded_t(mut self, radius: impl Into<Length>) -> Self {
        let Length::Px(r) = radius.into() else { return self };
        self.style_mut().border_radius_tl = Some(r);
        self.style_mut().border_radius_tr = Some(r);
        self
    }
    fn rounded_b(mut self, radius: impl Into<Length>) -> Self {
        let Length::Px(r) = radius.into() else { return self };
        self.style_mut().border_radius_bl = Some(r);
        self.style_mut().border_radius_br = Some(r);
        self
    }
    fn rounded_full(mut self) -> Self {
        let s = self.style_mut();
        s.border_radius_tl = Some(9999.0); s.border_radius_tr = Some(9999.0);
        s.border_radius_bl = Some(9999.0); s.border_radius_br = Some(9999.0);
        self
    }

    fn bg(mut self, color: impl Into<Color>) -> Self {
        self.style_mut().background = Some(color.into()); self
    }

    fn shadow(mut self, shadow: BoxShadowStyle) -> Self {
        self.style_mut().box_shadows.push(shadow); self
    }

    fn opacity(mut self, val: f32) -> Self {
        self.style_mut().opacity = Some(val); self
    }

    fn overflow_hidden(mut self) -> Self {
        self.style_mut().overflow_x = Some(Overflow::Hidden);
        self.style_mut().overflow_y = Some(Overflow::Hidden);
        self
    }
    fn overflow_scroll(mut self) -> Self {
        self.style_mut().overflow_x = Some(Overflow::Scroll);
        self.style_mut().overflow_y = Some(Overflow::Scroll);
        self
    }
    fn overflow_y_scroll(mut self) -> Self {
        self.style_mut().overflow_y = Some(Overflow::Scroll); self
    }

    fn cursor_pointer(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::Pointer); self
    }
    fn cursor_text(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::Text); self
    }
    fn cursor_grab(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::Grab); self
    }
    fn cursor_not_allowed(mut self) -> Self {
        self.style_mut().cursor = Some(CursorStyle::NotAllowed); self
    }

    fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.style_mut().text_color = Some(color.into()); self
    }
    fn text_size(mut self, size: impl Into<Length>) -> Self {
        let Length::Px(s) = size.into() else { return self };
        self.style_mut().font_size = Some(s); self
    }
    fn text_xs(mut self) -> Self { self.style_mut().font_size = Some(12.0); self }
    fn text_sm(mut self) -> Self { self.style_mut().font_size = Some(14.0); self }
    fn text_base(mut self) -> Self { self.style_mut().font_size = Some(16.0); self }
    fn text_lg(mut self) -> Self { self.style_mut().font_size = Some(18.0); self }
    fn text_xl(mut self) -> Self { self.style_mut().font_size = Some(20.0); self }

    fn font_weight(mut self, weight: FontWeight) -> Self {
        self.style_mut().font_weight = Some(weight); self
    }
    fn line_height(mut self, val: f32) -> Self {
        self.style_mut().line_height = Some(val); self
    }
    fn letter_spacing(mut self, val: f32) -> Self {
        self.style_mut().letter_spacing = Some(val); self
    }
    fn text_align(mut self, align: TextAlign) -> Self {
        self.style_mut().text_align = Some(align); self
    }
    fn text_ellipsis(mut self) -> Self {
        self.style_mut().text_overflow = Some(TextOverflow::Ellipsis); self
    }
    fn text_wrap(mut self) -> Self {
        self.style_mut().text_overflow = Some(TextOverflow::Wrap); self
    }
    fn text_nowrap(mut self) -> Self {
        self.style_mut().text_overflow = Some(TextOverflow::NoWrap); self
    }
    fn text_decoration(mut self, decoration: TextDecoration) -> Self {
        self.style_mut().text_decoration = Some(decoration); self
    }

    fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        if condition { then(self) } else { self }
    }

    fn apply(mut self, refinement: &Style) -> Self {
        self.style_mut().merge(refinement);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;

    struct TestBox { style: Style }
    impl TestBox { fn new() -> Self { Self { style: Style::new() } } }
    impl Styled for TestBox {
        fn style_mut(&mut self) -> &mut Style { &mut self.style }
    }

    #[test]
    fn flex_row_sets_display_and_direction() {
        let b = TestBox::new().flex_row();
        assert_eq!(b.style.display, Some(Display::Flex));
        assert_eq!(b.style.flex_direction, Some(FlexDirection::Row));
    }

    #[test]
    fn chained_sizing() {
        let b = TestBox::new().w(px(100.0)).h(px(50.0));
        assert_eq!(b.style.width, Some(Length::Px(100.0)));
        assert_eq!(b.style.height, Some(Length::Px(50.0)));
    }

    #[test]
    fn padding_shorthand() {
        let b = TestBox::new().p(px(16.0));
        assert_eq!(b.style.padding_top, Some(Length::Px(16.0)));
        assert_eq!(b.style.padding_right, Some(Length::Px(16.0)));
        assert_eq!(b.style.padding_bottom, Some(Length::Px(16.0)));
        assert_eq!(b.style.padding_left, Some(Length::Px(16.0)));
    }

    #[test]
    fn flex_1_sets_grow_shrink_basis() {
        let b = TestBox::new().flex_1();
        assert_eq!(b.style.flex_grow, Some(1.0));
        assert_eq!(b.style.flex_shrink, Some(1.0));
        assert_eq!(b.style.flex_basis, Some(Length::Px(0.0)));
    }

    #[test]
    fn conditional_when() {
        let b = TestBox::new()
            .bg(Color::rgb(255, 0, 0))
            .when(true, |s| s.opacity(0.5))
            .when(false, |s| s.hidden());
        assert_eq!(b.style.opacity, Some(0.5));
        assert_ne!(b.style.display, Some(Display::None));
    }

    #[test]
    fn rounded_full() {
        let b = TestBox::new().rounded_full();
        assert_eq!(b.style.border_radius_tl, Some(9999.0));
        assert_eq!(b.style.border_radius_br, Some(9999.0));
    }

    #[test]
    fn grid_cols_count() {
        let b = TestBox::new().grid().grid_cols_count(3);
        let tpl = b.style.grid_template.unwrap();
        assert_eq!(tpl.columns.len(), 3);
        assert_eq!(tpl.columns[0], TrackSize::Fr(1.0));
    }

    #[test]
    fn size_full_shorthand() {
        let b = TestBox::new().size_full();
        assert_eq!(b.style.width, Some(Length::Pct(100.0)));
        assert_eq!(b.style.height, Some(Length::Pct(100.0)));
    }
}
```

**Step 2: Update lib.rs** — add `mod styled; pub use styled::Styled;`

**Step 3: Verify**

Run: `cargo test -p velox-ui`
Expected: All tests pass

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(velox-ui): Styled trait with full fluent builder API"
```

---

## Task 4: Element Trait + IntoElement + AnyElement

**Files:**
- Create: `crates/velox-ui/src/element.rs`
- Modify: `crates/velox-ui/src/lib.rs`

**Step 1: Write Element types**

```rust
use std::any::TypeId;

pub type ElementKey = u64;

pub trait Element: 'static {
    type State: 'static + Default;

    fn element_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn layout(
        &mut self,
        state: &mut Self::State,
        children: &[AnyElement],
        cx: &mut LayoutContext,
    ) -> LayoutRequest;

    fn paint(
        &mut self,
        state: &mut Self::State,
        bounds: velox_scene::Rect,
        cx: &mut PaintContext,
    );
}

pub struct LayoutRequest {
    pub taffy_style: taffy::Style,
}

pub struct LayoutContext<'a> {
    pub(crate) taffy: &'a mut taffy::TaffyTree<()>,
}

pub struct PaintContext<'a> {
    pub(crate) commands: &'a mut velox_scene::CommandList,
    pub(crate) theme: &'a velox_style::Theme,
}

impl<'a> PaintContext<'a> {
    pub fn commands(&mut self) -> &mut velox_scene::CommandList {
        self.commands
    }

    pub fn theme(&self) -> &velox_style::Theme {
        self.theme
    }
}

pub trait IntoElement {
    type Element: Element;
    fn into_element(self) -> Self::Element;
    fn key(self, key: ElementKey) -> Keyed<Self>
    where
        Self: Sized,
    {
        Keyed { inner: self, key }
    }
}

pub struct Keyed<E> {
    pub(crate) inner: E,
    pub(crate) key: ElementKey,
}

impl<E: IntoElement> IntoElement for Keyed<E> {
    type Element = E::Element;
    fn into_element(self) -> Self::Element {
        self.inner.into_element()
    }
}

pub struct AnyElement {
    element: Box<dyn AnyElementTrait>,
    pub(crate) key: Option<ElementKey>,
    pub(crate) children: Vec<AnyElement>,
}

trait AnyElementTrait: 'static {
    fn element_type_id(&self) -> TypeId;
    fn layout_any(
        &mut self,
        children: &[AnyElement],
        cx: &mut LayoutContext,
    ) -> LayoutRequest;
    fn paint_any(
        &mut self,
        bounds: velox_scene::Rect,
        cx: &mut PaintContext,
    );
    fn style(&self) -> &crate::style::Style;
}

struct TypedElement<E: Element> {
    element: E,
    state: E::State,
}

impl<E: Element> AnyElementTrait for TypedElement<E>
where
    E: HasStyle,
{
    fn element_type_id(&self) -> TypeId {
        self.element.element_type_id()
    }

    fn layout_any(
        &mut self,
        children: &[AnyElement],
        cx: &mut LayoutContext,
    ) -> LayoutRequest {
        self.element.layout(&mut self.state, children, cx)
    }

    fn paint_any(
        &mut self,
        bounds: velox_scene::Rect,
        cx: &mut PaintContext,
    ) {
        self.element.paint(&mut self.state, bounds, cx);
    }

    fn style(&self) -> &crate::style::Style {
        self.element.get_style()
    }
}

pub trait HasStyle {
    fn get_style(&self) -> &crate::style::Style;
}

impl AnyElement {
    pub fn new<E: Element + HasStyle>(element: E, key: Option<ElementKey>, children: Vec<AnyElement>) -> Self {
        Self {
            element: Box::new(TypedElement {
                element,
                state: E::State::default(),
            }),
            key,
            children,
        }
    }

    pub fn element_type_id(&self) -> TypeId {
        self.element.element_type_id()
    }

    pub fn layout(&mut self, cx: &mut LayoutContext) -> LayoutRequest {
        self.element.layout_any(&self.children, cx)
    }

    pub fn paint(&mut self, bounds: velox_scene::Rect, cx: &mut PaintContext) {
        self.element.paint_any(bounds, cx);
    }

    pub fn style(&self) -> &crate::style::Style {
        self.element.style()
    }

    pub fn children(&self) -> &[AnyElement] {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<AnyElement> {
        &mut self.children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_request_created() {
        let req = LayoutRequest {
            taffy_style: taffy::Style::default(),
        };
        assert_eq!(req.taffy_style.display, taffy::Display::Flex);
    }

    #[test]
    fn element_key_wrapping() {
        struct DummyElement;
        struct DummyEl;
        impl Element for DummyEl {
            type State = ();
            fn layout(&mut self, _: &mut (), _: &[AnyElement], _: &mut LayoutContext) -> LayoutRequest {
                LayoutRequest { taffy_style: taffy::Style::default() }
            }
            fn paint(&mut self, _: &mut (), _: velox_scene::Rect, _: &mut PaintContext) {}
        }
        impl IntoElement for DummyElement {
            type Element = DummyEl;
            fn into_element(self) -> DummyEl { DummyEl }
        }

        let keyed = DummyElement.key(42);
        assert_eq!(keyed.key, 42);
    }
}
```

**Step 2: Update lib.rs** — add `mod element; pub use element::*;`

**Step 3: Verify**

Run: `cargo test -p velox-ui`

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(velox-ui): Element trait, IntoElement, AnyElement"
```

---

## Task 5: ParentElement Trait

**Files:**
- Create: `crates/velox-ui/src/parent.rs`
- Modify: `crates/velox-ui/src/lib.rs`

**Step 1: Write ParentElement**

```rust
use crate::element::{AnyElement, IntoElement};

pub trait ParentElement: Sized {
    fn children_mut(&mut self) -> &mut Vec<AnyElement>;

    fn child(mut self, child: impl IntoAnyElement) -> Self {
        let any = child.into_any_element();
        self.children_mut().push(any);
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = impl IntoAnyElement>) -> Self {
        for child in children {
            self.children_mut().push(child.into_any_element());
        }
        self
    }
}

pub trait IntoAnyElement {
    fn into_any_element(self) -> AnyElement;
}
```

Provide `IntoAnyElement` blanket impls once elements exist (Task 7+). For now, the trait is defined.

**Step 2: Update lib.rs**

**Step 3: Verify + Commit**

```bash
git add -A
git commit -m "feat(velox-ui): ParentElement trait for child composition"
```

---

## Task 6: InteractiveElement Trait

**Files:**
- Create: `crates/velox-ui/src/interactive.rs`
- Modify: `crates/velox-ui/src/lib.rs`

**Step 1: Write event types and trait**

```rust
use velox_scene::{MouseEvent, KeyEvent, ScrollEvent, Point};
use std::sync::Arc;

pub type ClickHandler = Arc<dyn Fn(&ClickEvent) + 'static>;
pub type MouseHandler = Arc<dyn Fn(&MouseEvent) + 'static>;
pub type ScrollHandler = Arc<dyn Fn(&ScrollEvent) + 'static>;
pub type KeyHandler = Arc<dyn Fn(&KeyEvent) + 'static>;
pub type HoverHandler = Arc<dyn Fn(bool) + 'static>;
pub type FocusHandler = Arc<dyn Fn(bool) + 'static>;

#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub position: Point,
    pub button: velox_scene::MouseButton,
    pub click_count: u32,
}

pub struct EventHandlers {
    pub on_click: Option<ClickHandler>,
    pub on_mouse_down: Option<MouseHandler>,
    pub on_mouse_up: Option<MouseHandler>,
    pub on_hover: Option<HoverHandler>,
    pub on_scroll: Option<ScrollHandler>,
    pub on_key_down: Option<KeyHandler>,
    pub on_focus: Option<FocusHandler>,
    pub focusable: bool,
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self {
            on_click: None,
            on_mouse_down: None,
            on_mouse_up: None,
            on_hover: None,
            on_scroll: None,
            on_key_down: None,
            on_focus: None,
            focusable: false,
        }
    }
}

pub trait InteractiveElement: Sized {
    fn handlers_mut(&mut self) -> &mut EventHandlers;

    fn on_click(mut self, handler: impl Fn(&ClickEvent) + 'static) -> Self {
        self.handlers_mut().on_click = Some(Arc::new(handler));
        self
    }

    fn on_mouse_down(mut self, handler: impl Fn(&MouseEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_down = Some(Arc::new(handler));
        self
    }

    fn on_mouse_up(mut self, handler: impl Fn(&MouseEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_up = Some(Arc::new(handler));
        self
    }

    fn on_hover(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.handlers_mut().on_hover = Some(Arc::new(handler));
        self
    }

    fn on_scroll(mut self, handler: impl Fn(&ScrollEvent) + 'static) -> Self {
        self.handlers_mut().on_scroll = Some(Arc::new(handler));
        self
    }

    fn on_key_down(mut self, handler: impl Fn(&KeyEvent) + 'static) -> Self {
        self.handlers_mut().on_key_down = Some(Arc::new(handler));
        self
    }

    fn on_focus(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.handlers_mut().on_focus = Some(Arc::new(handler));
        self
    }

    fn focusable(mut self) -> Self {
        self.handlers_mut().focusable = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    struct TestInteractive {
        handlers: EventHandlers,
    }
    impl TestInteractive {
        fn new() -> Self { Self { handlers: EventHandlers::default() } }
    }
    impl InteractiveElement for TestInteractive {
        fn handlers_mut(&mut self) -> &mut EventHandlers { &mut self.handlers }
    }

    #[test]
    fn on_click_registers_handler() {
        let el = TestInteractive::new().on_click(|_| {});
        assert!(el.handlers.on_click.is_some());
    }

    #[test]
    fn focusable_sets_flag() {
        let el = TestInteractive::new().focusable();
        assert!(el.handlers.focusable);
    }

    #[test]
    fn default_handlers_all_none() {
        let h = EventHandlers::default();
        assert!(h.on_click.is_none());
        assert!(h.on_mouse_down.is_none());
        assert!(!h.focusable);
    }
}
```

**Step 2: Update lib.rs, verify, commit**

```bash
git add -A
git commit -m "feat(velox-ui): InteractiveElement trait with event handlers"
```

---

## Task 7: Taffy Layout Engine

**Files:**
- Create: `crates/velox-ui/src/layout_engine.rs`
- Modify: `crates/velox-ui/src/lib.rs`

Bridges the `Style` struct to Taffy's CSS layout engine.

**Step 1: Write Style → taffy::Style conversion**

```rust
use taffy::prelude::*;
use crate::length::Length;
use crate::style::{self, Display, FlexDirection, FlexWrap, AlignItems, JustifyContent, Position, TrackSize};

pub struct LayoutEngine {
    pub(crate) taffy: TaffyTree<()>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    pub fn new_leaf(&mut self, style: &crate::style::Style) -> TaffyResult<NodeId> {
        let ts = convert_style(style);
        self.taffy.new_leaf(ts)
    }

    pub fn new_with_children(
        &mut self,
        style: &crate::style::Style,
        children: &[NodeId],
    ) -> TaffyResult<NodeId> {
        let ts = convert_style(style);
        self.taffy.new_with_children(ts, children)
    }

    pub fn set_style(&mut self, node: NodeId, style: &crate::style::Style) -> TaffyResult<()> {
        let ts = convert_style(style);
        self.taffy.set_style(node, ts)
    }

    pub fn remove(&mut self, node: NodeId) -> TaffyResult<NodeId> {
        self.taffy.remove(node)
    }

    pub fn compute_layout(&mut self, root: NodeId, available: Size<AvailableSpace>) -> TaffyResult<()> {
        self.taffy.compute_layout(root, available)
    }

    pub fn layout(&self, node: NodeId) -> TaffyResult<&Layout> {
        self.taffy.layout(node)
    }
}

fn convert_length(len: &Length) -> taffy::LengthPercentageAuto {
    match len {
        Length::Px(v) => taffy::LengthPercentageAuto::Length(*v),
        Length::Pct(v) => taffy::LengthPercentageAuto::Percent(*v / 100.0),
        Length::Auto | Length::Fr(_) => taffy::LengthPercentageAuto::Auto,
    }
}

fn convert_length_dimension(len: &Length) -> taffy::Dimension {
    match len {
        Length::Px(v) => taffy::Dimension::Length(*v),
        Length::Pct(v) => taffy::Dimension::Percent(*v / 100.0),
        Length::Auto | Length::Fr(_) => taffy::Dimension::Auto,
    }
}

fn convert_length_lp(len: &Length) -> taffy::LengthPercentage {
    match len {
        Length::Px(v) => taffy::LengthPercentage::Length(*v),
        Length::Pct(v) => taffy::LengthPercentage::Percent(*v / 100.0),
        Length::Auto | Length::Fr(_) => taffy::LengthPercentage::Length(0.0),
    }
}

pub fn convert_style(style: &crate::style::Style) -> taffy::Style {
    let mut ts = taffy::Style::default();

    if let Some(display) = &style.display {
        ts.display = match display {
            Display::Flex => taffy::Display::Flex,
            Display::Grid => taffy::Display::Grid,
            Display::Block => taffy::Display::Block,
            Display::None => taffy::Display::None,
        };
    }

    if let Some(pos) = &style.position {
        ts.position = match pos {
            Position::Relative => taffy::Position::Relative,
            Position::Absolute => taffy::Position::Absolute,
        };
    }

    if let Some(v) = &style.width { ts.size.width = convert_length_dimension(v); }
    if let Some(v) = &style.height { ts.size.height = convert_length_dimension(v); }
    if let Some(v) = &style.min_width { ts.min_size.width = convert_length_dimension(v); }
    if let Some(v) = &style.min_height { ts.min_size.height = convert_length_dimension(v); }
    if let Some(v) = &style.max_width { ts.max_size.width = convert_length_dimension(v); }
    if let Some(v) = &style.max_height { ts.max_size.height = convert_length_dimension(v); }
    if let Some(v) = &style.aspect_ratio { ts.aspect_ratio = Some(*v); }

    if let Some(v) = &style.padding_top { ts.padding.top = convert_length_lp(v); }
    if let Some(v) = &style.padding_right { ts.padding.right = convert_length_lp(v); }
    if let Some(v) = &style.padding_bottom { ts.padding.bottom = convert_length_lp(v); }
    if let Some(v) = &style.padding_left { ts.padding.left = convert_length_lp(v); }

    if let Some(v) = &style.margin_top { ts.margin.top = convert_length(v); }
    if let Some(v) = &style.margin_right { ts.margin.right = convert_length(v); }
    if let Some(v) = &style.margin_bottom { ts.margin.bottom = convert_length(v); }
    if let Some(v) = &style.margin_left { ts.margin.left = convert_length(v); }

    if let Some(v) = &style.inset_top { ts.inset.top = convert_length(v); }
    if let Some(v) = &style.inset_right { ts.inset.right = convert_length(v); }
    if let Some(v) = &style.inset_bottom { ts.inset.bottom = convert_length(v); }
    if let Some(v) = &style.inset_left { ts.inset.left = convert_length(v); }

    if let Some(v) = &style.flex_direction {
        ts.flex_direction = match v {
            FlexDirection::Row => taffy::FlexDirection::Row,
            FlexDirection::Column => taffy::FlexDirection::Column,
            FlexDirection::RowReverse => taffy::FlexDirection::RowReverse,
            FlexDirection::ColumnReverse => taffy::FlexDirection::ColumnReverse,
        };
    }
    if let Some(v) = &style.flex_wrap {
        ts.flex_wrap = match v {
            FlexWrap::NoWrap => taffy::FlexWrap::NoWrap,
            FlexWrap::Wrap => taffy::FlexWrap::Wrap,
            FlexWrap::WrapReverse => taffy::FlexWrap::WrapReverse,
        };
    }
    if let Some(v) = style.flex_grow { ts.flex_grow = v; }
    if let Some(v) = style.flex_shrink { ts.flex_shrink = v; }
    if let Some(v) = &style.flex_basis { ts.flex_basis = convert_length_dimension(v); }

    if let Some(v) = &style.align_items {
        ts.align_items = Some(convert_align(*v));
    }
    if let Some(v) = &style.align_self {
        ts.align_self = Some(convert_align(*v));
    }
    if let Some(v) = &style.justify_content {
        ts.justify_content = Some(convert_justify(*v));
    }

    if let Some(v) = &style.gap {
        let g = convert_length_lp(v);
        ts.gap = Size { width: g, height: g };
    }
    if let Some(v) = &style.column_gap {
        ts.gap.width = convert_length_lp(v);
    }
    if let Some(v) = &style.row_gap {
        ts.gap.height = convert_length_lp(v);
    }

    if let Some(v) = &style.border_top_width {
        ts.border.top = taffy::LengthPercentage::Length(*v);
    }
    if let Some(v) = &style.border_right_width {
        ts.border.right = taffy::LengthPercentage::Length(*v);
    }
    if let Some(v) = &style.border_bottom_width {
        ts.border.bottom = taffy::LengthPercentage::Length(*v);
    }
    if let Some(v) = &style.border_left_width {
        ts.border.left = taffy::LengthPercentage::Length(*v);
    }

    ts
}

fn convert_align(a: AlignItems) -> taffy::AlignItems {
    match a {
        AlignItems::Start => taffy::AlignItems::FlexStart,
        AlignItems::Center => taffy::AlignItems::Center,
        AlignItems::End => taffy::AlignItems::FlexEnd,
        AlignItems::Stretch => taffy::AlignItems::Stretch,
        AlignItems::Baseline => taffy::AlignItems::Baseline,
    }
}

fn convert_justify(j: JustifyContent) -> taffy::JustifyContent {
    match j {
        JustifyContent::Start => taffy::JustifyContent::FlexStart,
        JustifyContent::Center => taffy::JustifyContent::Center,
        JustifyContent::End => taffy::JustifyContent::FlexEnd,
        JustifyContent::SpaceBetween => taffy::JustifyContent::SpaceBetween,
        JustifyContent::SpaceAround => taffy::JustifyContent::SpaceAround,
        JustifyContent::SpaceEvenly => taffy::JustifyContent::SpaceEvenly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;
    use crate::style::Style;

    #[test]
    fn convert_flex_row_with_gap() {
        let mut style = Style::new();
        style.display = Some(Display::Flex);
        style.flex_direction = Some(FlexDirection::Row);
        style.gap = Some(Length::Px(8.0));

        let ts = convert_style(&style);
        assert_eq!(ts.display, taffy::Display::Flex);
        assert_eq!(ts.flex_direction, taffy::FlexDirection::Row);
    }

    #[test]
    fn convert_size_constraints() {
        let mut style = Style::new();
        style.width = Some(Length::Px(100.0));
        style.min_height = Some(Length::Px(50.0));
        style.max_width = Some(Length::Pct(80.0));

        let ts = convert_style(&style);
        assert_eq!(ts.size.width, taffy::Dimension::Length(100.0));
        assert_eq!(ts.min_size.height, taffy::Dimension::Length(50.0));
        assert_eq!(ts.max_size.width, taffy::Dimension::Percent(0.8));
    }

    #[test]
    fn layout_engine_computes_flex_row() {
        let mut engine = LayoutEngine::new();
        let mut parent_style = Style::new();
        parent_style.display = Some(Display::Flex);
        parent_style.flex_direction = Some(FlexDirection::Row);
        parent_style.width = Some(Length::Px(200.0));
        parent_style.height = Some(Length::Px(100.0));

        let mut child_style = Style::new();
        child_style.width = Some(Length::Px(80.0));
        child_style.height = Some(Length::Px(40.0));

        let child1 = engine.new_leaf(&child_style).unwrap();
        let child2 = engine.new_leaf(&child_style).unwrap();
        let root = engine.new_with_children(&parent_style, &[child1, child2]).unwrap();

        engine.compute_layout(root, Size {
            width: AvailableSpace::Definite(200.0),
            height: AvailableSpace::Definite(100.0),
        }).unwrap();

        let layout1 = engine.layout(child1).unwrap();
        let layout2 = engine.layout(child2).unwrap();

        assert_eq!(layout1.size.width, 80.0);
        assert_eq!(layout2.location.x, 80.0);
    }

    #[test]
    fn layout_engine_flex_1_fills_space() {
        let mut engine = LayoutEngine::new();
        let mut parent_style = Style::new();
        parent_style.display = Some(Display::Flex);
        parent_style.flex_direction = Some(FlexDirection::Row);
        parent_style.width = Some(Length::Px(300.0));
        parent_style.height = Some(Length::Px(100.0));

        let mut fixed_style = Style::new();
        fixed_style.width = Some(Length::Px(100.0));

        let mut flex_style = Style::new();
        flex_style.flex_grow = Some(1.0);
        flex_style.flex_shrink = Some(1.0);
        flex_style.flex_basis = Some(Length::Px(0.0));

        let fixed = engine.new_leaf(&fixed_style).unwrap();
        let flexible = engine.new_leaf(&flex_style).unwrap();
        let root = engine.new_with_children(&parent_style, &[fixed, flexible]).unwrap();

        engine.compute_layout(root, Size {
            width: AvailableSpace::Definite(300.0),
            height: AvailableSpace::Definite(100.0),
        }).unwrap();

        let flex_layout = engine.layout(flexible).unwrap();
        assert_eq!(flex_layout.size.width, 200.0);
    }

    #[test]
    fn layout_engine_padding() {
        let mut engine = LayoutEngine::new();
        let mut parent_style = Style::new();
        parent_style.display = Some(Display::Flex);
        parent_style.width = Some(Length::Px(200.0));
        parent_style.height = Some(Length::Px(200.0));
        parent_style.padding_top = Some(Length::Px(10.0));
        parent_style.padding_left = Some(Length::Px(20.0));

        let mut child_style = Style::new();
        child_style.width = Some(Length::Px(50.0));
        child_style.height = Some(Length::Px(50.0));

        let child = engine.new_leaf(&child_style).unwrap();
        let root = engine.new_with_children(&parent_style, &[child]).unwrap();

        engine.compute_layout(root, Size {
            width: AvailableSpace::Definite(200.0),
            height: AvailableSpace::Definite(200.0),
        }).unwrap();

        let layout = engine.layout(child).unwrap();
        assert_eq!(layout.location.x, 20.0);
        assert_eq!(layout.location.y, 10.0);
    }
}
```

**Step 2: Update lib.rs, verify, commit**

```bash
git add -A
git commit -m "feat(velox-ui): Taffy layout engine with Style conversion"
```

---

## Task 8: Div Element

**Files:**
- Create: `crates/velox-ui/src/elements/mod.rs`
- Create: `crates/velox-ui/src/elements/div.rs`
- Modify: `crates/velox-ui/src/lib.rs`

The `Div` is the universal container — like HTML's `<div>`. This is the single most important element.

**Step 1: Write Div**

```rust
use crate::element::{AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext};
use crate::interactive::{EventHandlers, InteractiveElement};
use crate::parent::{ParentElement, IntoAnyElement};
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{Color, CommandList, Point, Rect};

pub struct Div {
    pub(crate) style: Style,
    pub(crate) hover_style: Option<Style>,
    pub(crate) active_style: Option<Style>,
    pub(crate) handlers: EventHandlers,
    pub(crate) children: Vec<AnyElement>,
}

pub fn div() -> Div {
    Div {
        style: Style::new(),
        hover_style: None,
        active_style: None,
        handlers: EventHandlers::default(),
        children: Vec::new(),
    }
}

impl Div {
    pub fn hover(mut self, f: impl FnOnce(StyleBuilder) -> StyleBuilder) -> Self {
        let builder = f(StyleBuilder(Style::new()));
        self.hover_style = Some(builder.0);
        self
    }

    pub fn active(mut self, f: impl FnOnce(StyleBuilder) -> StyleBuilder) -> Self {
        let builder = f(StyleBuilder(Style::new()));
        self.active_style = Some(builder.0);
        self
    }
}

pub struct StyleBuilder(pub Style);

impl Styled for StyleBuilder {
    fn style_mut(&mut self) -> &mut Style { &mut self.0 }
}

impl Styled for Div {
    fn style_mut(&mut self) -> &mut Style { &mut self.style }
}

impl InteractiveElement for Div {
    fn handlers_mut(&mut self) -> &mut EventHandlers { &mut self.handlers }
}

impl ParentElement for Div {
    fn children_mut(&mut self) -> &mut Vec<AnyElement> { &mut self.children }
}

impl HasStyle for Div {
    fn get_style(&self) -> &Style { &self.style }
}

#[derive(Default)]
pub struct DivState;

impl Element for Div {
    type State = DivState;

    fn layout(
        &mut self,
        _state: &mut DivState,
        _children: &[AnyElement],
        _cx: &mut LayoutContext,
    ) -> LayoutRequest {
        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(
        &mut self,
        _state: &mut DivState,
        bounds: Rect,
        cx: &mut PaintContext,
    ) {
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }
        if let Some(bc) = self.style.border_color {
            let bw = self.style.border_top_width.unwrap_or(0.0);
            if bw > 0.0 {
                cx.commands().stroke_rect(bounds, bc, bw);
            }
        }
        for shadow in &self.style.box_shadows {
            cx.commands().box_shadow(
                bounds,
                shadow.color,
                shadow.blur_radius,
                Point::new(shadow.offset_x, shadow.offset_y),
                shadow.spread,
            );
        }
    }
}

impl IntoElement for Div {
    type Element = Div;
    fn into_element(self) -> Div { self }
}

impl IntoAnyElement for Div {
    fn into_any_element(self) -> AnyElement {
        let children = std::mem::replace(&mut Vec::new(), self.children);
        // Move children out before boxing
        let mut d = self;
        let children = std::mem::take(&mut d.children);
        AnyElement::new(d, None, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;

    #[test]
    fn div_fluent_styling() {
        let d = div()
            .flex_row()
            .gap(px(8.0))
            .p(px(16.0))
            .bg(Color::rgb(255, 0, 0));

        assert_eq!(d.style.display, Some(crate::style::Display::Flex));
        assert_eq!(d.style.background, Some(Color::rgb(255, 0, 0)));
    }

    #[test]
    fn div_hover_style() {
        let d = div()
            .bg(Color::rgb(0, 0, 0))
            .hover(|s| s.bg(Color::rgb(50, 50, 50)));

        assert!(d.hover_style.is_some());
        assert_eq!(d.hover_style.unwrap().background, Some(Color::rgb(50, 50, 50)));
    }

    #[test]
    fn div_with_event_handler() {
        let d = div()
            .on_click(|_| {})
            .cursor_pointer();

        assert!(d.handlers.on_click.is_some());
        assert_eq!(d.style.cursor, Some(crate::style::CursorStyle::Pointer));
    }

    #[test]
    fn div_paint_emits_fill_rect() {
        let mut d = div().bg(Color::rgb(255, 0, 0));
        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
        };
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let mut state = DivState;
        d.paint(&mut state, bounds, &mut cx);

        assert_eq!(commands.commands().len(), 1);
        assert!(matches!(
            commands.commands()[0],
            velox_scene::PaintCommand::FillRect { .. }
        ));
    }
}
```

**Step 2: Create `elements/mod.rs`:**

```rust
mod div;
pub use div::{div, Div, StyleBuilder};
```

**Step 3: Update lib.rs — add `mod elements; pub use elements::*;`**

**Step 4: Verify + commit**

```bash
git add -A
git commit -m "feat(velox-ui): Div element with styling, events, children"
```

---

## Task 9: Text Element

**Files:**
- Create: `crates/velox-ui/src/elements/text.rs`
- Modify: `crates/velox-ui/src/elements/mod.rs`

**Step 1: Write TextElement**

The text element renders a string. It uses velox-scene's `PaintCommand::FillRect` as placeholder until cosmic-text glyph rendering is wired via `DrawGlyphs`. The important thing is the API shape + layout integration.

```rust
use crate::element::{AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::Rect;

pub struct TextElement {
    pub(crate) content: String,
    pub(crate) style: Style,
}

pub fn text(content: impl Into<String>) -> TextElement {
    TextElement {
        content: content.into(),
        style: Style::new(),
    }
}

impl Styled for TextElement {
    fn style_mut(&mut self) -> &mut Style { &mut self.style }
}

impl HasStyle for TextElement {
    fn get_style(&self) -> &Style { &self.style }
}

#[derive(Default)]
pub struct TextState;

impl Element for TextElement {
    type State = TextState;

    fn layout(
        &mut self,
        _state: &mut TextState,
        _children: &[AnyElement],
        _cx: &mut LayoutContext,
    ) -> LayoutRequest {
        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(
        &mut self,
        _state: &mut TextState,
        bounds: Rect,
        cx: &mut PaintContext,
    ) {
        let color = self.style.text_color.unwrap_or(
            velox_scene::Color::rgb(0, 0, 0)
        );
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }
        // Text rendering will be wired to cosmic-text via DrawGlyphs
        // For now we emit a placeholder fill to mark the text bounds
        cx.commands().fill_rect(
            Rect::new(bounds.x, bounds.y, bounds.width.min(self.content.len() as f32 * 8.0), bounds.height.min(16.0)),
            color,
        );
    }
}

impl IntoElement for TextElement {
    type Element = TextElement;
    fn into_element(self) -> TextElement { self }
}

impl IntoAnyElement for TextElement {
    fn into_any_element(self) -> crate::element::AnyElement {
        crate::element::AnyElement::new(self, None, vec![])
    }
}

impl IntoElement for &str {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        text(self)
    }
}

impl IntoElement for String {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        text(self)
    }
}

impl IntoAnyElement for &str {
    fn into_any_element(self) -> crate::element::AnyElement {
        text(self).into_any_element()
    }
}

impl IntoAnyElement for String {
    fn into_any_element(self) -> crate::element::AnyElement {
        text(self).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;
    use crate::style::FontWeight;

    #[test]
    fn text_from_str() {
        let t = text("Hello");
        assert_eq!(t.content, "Hello");
    }

    #[test]
    fn text_styled() {
        let t = text("Hello")
            .text_lg()
            .font_weight(FontWeight::Bold)
            .text_color(velox_scene::Color::rgb(255, 255, 255));

        assert_eq!(t.style.font_size, Some(18.0));
        assert_eq!(t.style.font_weight, Some(FontWeight::Bold));
    }

    #[test]
    fn str_into_element() {
        let el: TextElement = "hello".into_element();
        assert_eq!(el.content, "hello");
    }

    #[test]
    fn string_into_element() {
        let el: TextElement = String::from("world").into_element();
        assert_eq!(el.content, "world");
    }
}
```

**Step 2: Update elements/mod.rs — add `mod text; pub use text::*;`**

**Step 3: Verify + commit**

```bash
git add -A
git commit -m "feat(velox-ui): Text element with IntoElement for strings"
```

---

## Task 10: Canvas Element

**Files:**
- Create: `crates/velox-ui/src/elements/canvas.rs`
- Modify: `crates/velox-ui/src/elements/mod.rs`

**Step 1: Write Canvas**

```rust
use crate::element::{AnyElement, Element, HasStyle, LayoutContext, LayoutRequest, PaintContext, IntoElement};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{CommandList, Rect};

type PaintCallback = Box<dyn Fn(Rect, &mut CommandList)>;

pub struct Canvas {
    style: Style,
    callback: PaintCallback,
}

pub fn canvas(callback: impl Fn(Rect, &mut CommandList) + 'static) -> Canvas {
    Canvas {
        style: Style::new(),
        callback: Box::new(callback),
    }
}

impl Styled for Canvas {
    fn style_mut(&mut self) -> &mut Style { &mut self.style }
}

impl HasStyle for Canvas {
    fn get_style(&self) -> &Style { &self.style }
}

#[derive(Default)]
pub struct CanvasState;

impl Element for Canvas {
    type State = CanvasState;

    fn layout(
        &mut self,
        _state: &mut CanvasState,
        _children: &[AnyElement],
        _cx: &mut LayoutContext,
    ) -> LayoutRequest {
        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(
        &mut self,
        _state: &mut CanvasState,
        bounds: Rect,
        cx: &mut PaintContext,
    ) {
        (self.callback)(bounds, cx.commands());
    }
}

impl IntoElement for Canvas {
    type Element = Canvas;
    fn into_element(self) -> Canvas { self }
}

impl IntoAnyElement for Canvas {
    fn into_any_element(self) -> crate::element::AnyElement {
        crate::element::AnyElement::new(self, None, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn canvas_calls_paint_callback() {
        let painted = Rc::new(Cell::new(false));
        let p = painted.clone();
        let c = canvas(move |bounds, commands| {
            p.set(true);
            commands.fill_rect(bounds, velox_scene::Color::rgb(255, 0, 0));
        })
        .w(px(100.0))
        .h(px(100.0));

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
        };
        let mut state = CanvasState;
        let mut canvas_el = c;
        canvas_el.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 100.0), &mut cx);

        assert!(painted.get());
        assert_eq!(commands.commands().len(), 1);
    }
}
```

**Step 2: Update elements/mod.rs, verify, commit**

```bash
git add -A
git commit -m "feat(velox-ui): Canvas element for custom paint commands"
```

---

## Task 11: Reconciler — Mount Phase

**Files:**
- Create: `crates/velox-ui/src/reconciler.rs`
- Modify: `crates/velox-ui/src/lib.rs`

The reconciler converts an element tree into `NodeTree` nodes + Taffy layout nodes. This task covers the mount (first render) phase.

**Step 1: Write reconciler types and mount**

```rust
use std::any::TypeId;
use velox_scene::{NodeId, NodeTree, Rect, Point};
use crate::element::{AnyElement, ElementKey};
use crate::layout_engine::LayoutEngine;
use crate::style::Style;

pub struct ReconcilerSlot {
    pub node_id: NodeId,
    pub taffy_node: taffy::NodeId,
    pub element_type: TypeId,
    pub key: Option<ElementKey>,
    pub style: Style,
    pub children: Vec<ReconcilerSlot>,
}

pub struct Reconciler {
    slots: Vec<ReconcilerSlot>,
}

impl Reconciler {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn mount(
        &mut self,
        elements: &mut [AnyElement],
        parent_node: Option<NodeId>,
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) -> Vec<taffy::NodeId> {
        let mut taffy_children = Vec::new();

        for element in elements.iter_mut() {
            let slot = self.mount_element(element, parent_node, tree, engine);
            taffy_children.push(slot.taffy_node);
            self.slots.push(slot);
        }

        taffy_children
    }

    fn mount_element(
        &self,
        element: &mut AnyElement,
        parent_node: Option<NodeId>,
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) -> ReconcilerSlot {
        let node_id = tree.insert(parent_node);
        let style = element.style().clone();

        let child_elements = element.children_mut();
        let mut child_slots = Vec::new();
        let mut child_taffy_nodes = Vec::new();

        for child in child_elements.iter_mut() {
            let child_slot = self.mount_element(child, Some(node_id), tree, engine);
            child_taffy_nodes.push(child_slot.taffy_node);
            child_slots.push(child_slot);
        }

        let taffy_node = if child_taffy_nodes.is_empty() {
            engine.new_leaf(&style).expect("taffy new_leaf")
        } else {
            engine
                .new_with_children(&style, &child_taffy_nodes)
                .expect("taffy new_with_children")
        };

        ReconcilerSlot {
            node_id,
            taffy_node,
            element_type: element.element_type_id(),
            key: element.key,
            style,
            children: child_slots,
        }
    }

    pub fn apply_layout(
        &self,
        engine: &LayoutEngine,
        tree: &mut NodeTree,
        parent_origin: Point,
    ) {
        for slot in &self.slots {
            self.apply_layout_slot(slot, engine, tree, parent_origin);
        }
    }

    fn apply_layout_slot(
        &self,
        slot: &ReconcilerSlot,
        engine: &LayoutEngine,
        tree: &mut NodeTree,
        parent_origin: Point,
    ) {
        let layout = engine.layout(slot.taffy_node).expect("layout exists");
        let rect = Rect::new(
            parent_origin.x + layout.location.x,
            parent_origin.y + layout.location.y,
            layout.size.width,
            layout.size.height,
        );
        tree.set_rect(slot.node_id, rect);

        let child_origin = Point::new(rect.x, rect.y);
        for child in &slot.children {
            self.apply_layout_slot(child, engine, tree, child_origin);
        }
    }

    pub fn slots(&self) -> &[ReconcilerSlot] {
        &self.slots
    }

    pub fn unmount(&self, tree: &mut NodeTree, engine: &mut LayoutEngine) {
        for slot in &self.slots {
            self.unmount_slot(slot, tree, engine);
        }
    }

    fn unmount_slot(&self, slot: &ReconcilerSlot, tree: &mut NodeTree, engine: &mut LayoutEngine) {
        for child in &slot.children {
            self.unmount_slot(child, tree, engine);
        }
        let _ = engine.remove(slot.taffy_node);
        tree.remove(slot.node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::div;
    use crate::styled::Styled;
    use crate::length::px;
    use crate::parent::ParentElement;

    #[test]
    fn mount_single_div() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 800.0, 600.0));

        let d = div().w(px(100.0)).h(px(50.0));
        let mut elements = vec![d.into_any_element()];

        let taffy_ids = reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);
        assert_eq!(taffy_ids.len(), 1);
        assert_eq!(reconciler.slots().len(), 1);
        assert!(tree.contains(reconciler.slots()[0].node_id));
    }

    #[test]
    fn mount_nested_div() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 800.0, 600.0));

        let d = div()
            .flex_row()
            .w(px(200.0))
            .h(px(100.0))
            .child(div().w(px(80.0)).h(px(40.0)))
            .child(div().w(px(80.0)).h(px(40.0)));

        let mut elements = vec![d.into_any_element()];
        reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);

        assert_eq!(reconciler.slots().len(), 1);
        assert_eq!(reconciler.slots()[0].children.len(), 2);
    }

    #[test]
    fn mount_and_compute_layout() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 400.0, 300.0));

        let d = div()
            .flex_row()
            .w(px(400.0))
            .h(px(300.0))
            .child(div().w(px(200.0)).h(px(100.0)))
            .child(div().w(px(200.0)).h(px(100.0)));

        let mut elements = vec![d.into_any_element()];
        let taffy_roots = reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);

        let root_taffy = taffy_roots[0];
        engine.compute_layout(root_taffy, taffy::prelude::Size {
            width: taffy::prelude::AvailableSpace::Definite(400.0),
            height: taffy::prelude::AvailableSpace::Definite(300.0),
        }).unwrap();

        reconciler.apply_layout(&engine, &mut tree, Point::new(0.0, 0.0));

        let child1_id = reconciler.slots()[0].children[0].node_id;
        let child2_id = reconciler.slots()[0].children[1].node_id;

        let r1 = tree.rect(child1_id).unwrap();
        let r2 = tree.rect(child2_id).unwrap();

        assert_eq!(r1.width, 200.0);
        assert_eq!(r2.x, 200.0);
    }

    #[test]
    fn unmount_removes_nodes() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        let d = div().w(px(100.0));
        let mut elements = vec![d.into_any_element()];
        reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);

        let mounted_id = reconciler.slots()[0].node_id;
        assert!(tree.contains(mounted_id));

        reconciler.unmount(&mut tree, &mut engine);
        assert!(!tree.contains(mounted_id));
    }
}
```

**Step 2: Update lib.rs, verify, commit**

```bash
git add -A
git commit -m "feat(velox-ui): Reconciler mount phase — element tree to NodeTree"
```

---

## Task 12: Reconciler — Diff & Patch Phase

**Files:**
- Modify: `crates/velox-ui/src/reconciler.rs`

Add diffing: compare new elements against existing slots, emit minimal patches.

**Step 1: Add Patch enum and diff logic**

Add to `reconciler.rs`:

```rust
#[derive(Debug)]
pub enum Patch {
    UpdateStyle { node_id: NodeId, taffy_node: taffy::NodeId, style: Style, layout_changed: bool },
    CreateNode { parent: NodeId, element_type: TypeId, key: Option<ElementKey>, style: Style },
    RemoveNode { node_id: NodeId, taffy_node: taffy::NodeId },
}

impl Reconciler {
    pub fn diff(
        &mut self,
        new_elements: &mut [AnyElement],
        parent_node: Option<NodeId>,
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) -> Vec<Patch> {
        let mut patches = Vec::new();
        let old_slots = std::mem::take(&mut self.slots);
        let mut new_slots = Vec::new();

        let max_len = old_slots.len().max(new_elements.len());

        for i in 0..max_len {
            match (old_slots.get(i), new_elements.get_mut(i)) {
                (Some(old), Some(new_el)) => {
                    if old.element_type == new_el.element_type_id() {
                        let new_style = new_el.style().clone();
                        let layout_changed = old.style.is_layout_affecting_different(&new_style);
                        if old.style != new_style {
                            patches.push(Patch::UpdateStyle {
                                node_id: old.node_id,
                                taffy_node: old.taffy_node,
                                style: new_style.clone(),
                                layout_changed,
                            });
                        }

                        let mut child_reconciler = Reconciler { slots: old.children.clone() };
                        let child_patches = child_reconciler.diff(
                            new_el.children_mut(),
                            Some(old.node_id),
                            tree,
                            engine,
                        );
                        patches.extend(child_patches);

                        new_slots.push(ReconcilerSlot {
                            node_id: old.node_id,
                            taffy_node: old.taffy_node,
                            element_type: old.element_type,
                            key: new_el.key,
                            style: new_el.style().clone(),
                            children: child_reconciler.slots,
                        });
                    } else {
                        self.collect_removes(&old, &mut patches);
                        let slot = self.mount_element(new_el, parent_node, tree, engine);
                        new_slots.push(slot);
                    }
                }
                (Some(old), None) => {
                    self.collect_removes(old, &mut patches);
                }
                (None, Some(new_el)) => {
                    let slot = self.mount_element(new_el, parent_node, tree, engine);
                    new_slots.push(slot);
                }
                (None, None) => break,
            }
        }

        self.slots = new_slots;
        patches
    }

    fn collect_removes(&self, slot: &ReconcilerSlot, patches: &mut Vec<Patch>) {
        for child in &slot.children {
            self.collect_removes(child, patches);
        }
        patches.push(Patch::RemoveNode {
            node_id: slot.node_id,
            taffy_node: slot.taffy_node,
        });
    }

    pub fn apply_patches(
        patches: &[Patch],
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) {
        for patch in patches {
            match patch {
                Patch::UpdateStyle { node_id, taffy_node, style, layout_changed } => {
                    engine.set_style(*taffy_node, style).ok();
                    if *layout_changed {
                        tree.mark_layout_dirty(*node_id);
                    } else {
                        tree.mark_paint_dirty(*node_id);
                    }
                }
                Patch::RemoveNode { node_id, taffy_node } => {
                    let _ = engine.remove(*taffy_node);
                    tree.remove(*node_id);
                }
                Patch::CreateNode { .. } => {
                    // Handled during mount in diff()
                }
            }
        }
    }
}

impl Clone for ReconcilerSlot {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            taffy_node: self.taffy_node,
            element_type: self.element_type,
            key: self.key,
            style: self.style.clone(),
            children: self.children.clone(),
        }
    }
}
```

**Step 2: Add diff tests**

```rust
#[test]
fn diff_updates_style() {
    let mut tree = NodeTree::new();
    let mut engine = LayoutEngine::new();
    let mut reconciler = Reconciler::new();

    let root = tree.insert(None);
    tree.set_rect(root, Rect::new(0.0, 0.0, 400.0, 300.0));

    let d1 = div().w(px(100.0)).h(px(50.0)).bg(velox_scene::Color::rgb(255, 0, 0));
    let mut elements1 = vec![d1.into_any_element()];
    reconciler.mount(&mut elements1, Some(root), &mut tree, &mut engine);

    let d2 = div().w(px(100.0)).h(px(50.0)).bg(velox_scene::Color::rgb(0, 255, 0));
    let mut elements2 = vec![d2.into_any_element()];
    let patches = reconciler.diff(&mut elements2, Some(root), &mut tree, &mut engine);

    assert!(!patches.is_empty());
    assert!(matches!(patches[0], Patch::UpdateStyle { layout_changed: false, .. }));
}

#[test]
fn diff_detects_layout_change() {
    let mut tree = NodeTree::new();
    let mut engine = LayoutEngine::new();
    let mut reconciler = Reconciler::new();

    let root = tree.insert(None);
    let d1 = div().w(px(100.0));
    let mut elements1 = vec![d1.into_any_element()];
    reconciler.mount(&mut elements1, Some(root), &mut tree, &mut engine);

    let d2 = div().w(px(200.0));
    let mut elements2 = vec![d2.into_any_element()];
    let patches = reconciler.diff(&mut elements2, Some(root), &mut tree, &mut engine);

    assert!(matches!(patches[0], Patch::UpdateStyle { layout_changed: true, .. }));
}

#[test]
fn diff_removes_extra_elements() {
    let mut tree = NodeTree::new();
    let mut engine = LayoutEngine::new();
    let mut reconciler = Reconciler::new();

    let root = tree.insert(None);
    let mut els = vec![
        div().into_any_element(),
        div().into_any_element(),
    ];
    reconciler.mount(&mut els, Some(root), &mut tree, &mut engine);
    assert_eq!(reconciler.slots().len(), 2);

    let mut els2 = vec![div().into_any_element()];
    let patches = reconciler.diff(&mut els2, Some(root), &mut tree, &mut engine);

    assert!(patches.iter().any(|p| matches!(p, Patch::RemoveNode { .. })));
    assert_eq!(reconciler.slots().len(), 1);
}

#[test]
fn diff_adds_new_elements() {
    let mut tree = NodeTree::new();
    let mut engine = LayoutEngine::new();
    let mut reconciler = Reconciler::new();

    let root = tree.insert(None);
    let mut els = vec![div().into_any_element()];
    reconciler.mount(&mut els, Some(root), &mut tree, &mut engine);

    let mut els2 = vec![
        div().into_any_element(),
        div().w(px(50.0)).into_any_element(),
    ];
    let _patches = reconciler.diff(&mut els2, Some(root), &mut tree, &mut engine);
    assert_eq!(reconciler.slots().len(), 2);
}
```

**Step 3: Verify + commit**

```bash
git add -A
git commit -m "feat(velox-ui): Reconciler diff/patch — style diffing, add/remove"
```

---

## Task 13: Component Trait + ViewContext

**Files:**
- Create: `crates/velox-ui/src/component.rs`
- Modify: `crates/velox-ui/src/lib.rs`

**Step 1: Write Component trait**

```rust
use crate::element::{AnyElement, IntoElement};
use crate::parent::IntoAnyElement;
use velox_style::Theme;

pub struct ViewContext<'a> {
    theme: &'a Theme,
}

impl<'a> ViewContext<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }
}

pub trait Component: 'static + Sized {
    fn render(&self, cx: &ViewContext) -> AnyElement;
}

pub struct ComponentHost<C: Component> {
    component: C,
}

impl<C: Component> ComponentHost<C> {
    pub fn new(component: C) -> Self {
        Self { component }
    }

    pub fn render(&self, cx: &ViewContext) -> AnyElement {
        self.component.render(cx)
    }

    pub fn component(&self) -> &C {
        &self.component
    }

    pub fn component_mut(&mut self) -> &mut C {
        &mut self.component
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::{div, text};
    use crate::styled::Styled;
    use crate::parent::ParentElement;
    use crate::length::px;

    struct Counter {
        count: u32,
    }

    impl Component for Counter {
        fn render(&self, cx: &ViewContext) -> AnyElement {
            div()
                .flex_row()
                .gap(px(8.0))
                .child(text(format!("Count: {}", self.count)))
                .into_any_element()
        }
    }

    #[test]
    fn component_renders_element_tree() {
        let counter = Counter { count: 42 };
        let host = ComponentHost::new(counter);
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);
        let element = host.render(&cx);
        assert!(!element.children().is_empty());
    }

    #[test]
    fn component_host_provides_access() {
        let counter = Counter { count: 0 };
        let mut host = ComponentHost::new(counter);
        host.component_mut().count = 10;
        assert_eq!(host.component().count, 10);
    }
}
```

**Step 2: Update lib.rs, verify, commit**

```bash
git add -A
git commit -m "feat(velox-ui): Component trait and ViewContext"
```

---

## Task 14: Img + Svg Stub Elements

**Files:**
- Create: `crates/velox-ui/src/elements/img.rs`
- Create: `crates/velox-ui/src/elements/svg.rs`
- Modify: `crates/velox-ui/src/elements/mod.rs`

Stub elements with correct API shapes. Full rendering will be wired when velox-media/velox-render are integrated.

**Step 1: Write Img element**

```rust
use crate::element::{AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext};
use crate::parent::IntoAnyElement;
use crate::style::{ObjectFit, Style};
use crate::styled::Styled;
use velox_scene::Rect;

#[derive(Debug, Clone)]
pub enum ImageSource {
    Path(String),
    Bytes(Vec<u8>),
    Url(String),
}

pub struct Img {
    source: ImageSource,
    style: Style,
    object_fit: ObjectFit,
}

pub fn img(source: ImageSource) -> Img {
    Img {
        source,
        style: Style::new(),
        object_fit: ObjectFit::Contain,
    }
}

impl Img {
    pub fn object_fit(mut self, fit: ObjectFit) -> Self {
        self.object_fit = fit;
        self
    }
}

impl Styled for Img {
    fn style_mut(&mut self) -> &mut Style { &mut self.style }
}

impl HasStyle for Img {
    fn get_style(&self) -> &Style { &self.style }
}

#[derive(Default)]
pub struct ImgState;

impl Element for Img {
    type State = ImgState;

    fn layout(&mut self, _: &mut ImgState, _: &[AnyElement], _: &mut LayoutContext) -> LayoutRequest {
        LayoutRequest { taffy_style: crate::layout_engine::convert_style(&self.style) }
    }

    fn paint(&mut self, _: &mut ImgState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }
        // TODO: Wire to TextureManager + DrawImage when velox-media ready
    }
}

impl IntoElement for Img {
    type Element = Img;
    fn into_element(self) -> Img { self }
}

impl IntoAnyElement for Img {
    fn into_any_element(self) -> AnyElement {
        AnyElement::new(self, None, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;

    #[test]
    fn img_from_path() {
        let i = img(ImageSource::Path("avatar.png".into()))
            .size(px(48.0))
            .rounded_full();
        assert_eq!(i.style.border_radius_tl, Some(9999.0));
    }

    #[test]
    fn img_object_fit() {
        let i = img(ImageSource::Path("photo.jpg".into()))
            .object_fit(ObjectFit::Cover);
        assert_eq!(i.object_fit, ObjectFit::Cover);
    }
}
```

**Step 2: Write Svg element** (similar structure, takes `&str` SVG data)

```rust
use crate::element::{AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::Rect;

pub struct Svg {
    data: String,
    style: Style,
}

pub fn svg(data: impl Into<String>) -> Svg {
    Svg {
        data: data.into(),
        style: Style::new(),
    }
}

impl Styled for Svg {
    fn style_mut(&mut self) -> &mut Style { &mut self.style }
}

impl HasStyle for Svg {
    fn get_style(&self) -> &Style { &self.style }
}

#[derive(Default)]
pub struct SvgState;

impl Element for Svg {
    type State = SvgState;

    fn layout(&mut self, _: &mut SvgState, _: &[AnyElement], _: &mut LayoutContext) -> LayoutRequest {
        LayoutRequest { taffy_style: crate::layout_engine::convert_style(&self.style) }
    }

    fn paint(&mut self, _: &mut SvgState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(color) = self.style.text_color {
            cx.commands().fill_rect(bounds, color);
        }
        // TODO: Wire SVG rasterizer
    }
}

impl IntoElement for Svg {
    type Element = Svg;
    fn into_element(self) -> Svg { self }
}

impl IntoAnyElement for Svg {
    fn into_any_element(self) -> AnyElement {
        AnyElement::new(self, None, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;

    #[test]
    fn svg_with_style() {
        let s = svg("<svg>...</svg>")
            .size(px(16.0))
            .text_color(velox_scene::Color::rgb(100, 100, 100));
        assert_eq!(s.style.text_color, Some(velox_scene::Color::rgb(100, 100, 100)));
    }
}
```

**Step 3: Update elements/mod.rs, verify, commit**

```bash
git add -A
git commit -m "feat(velox-ui): Img and Svg stub elements"
```

---

## Task 15: Workspace Integration + Facade Re-exports

**Files:**
- Modify: `Cargo.toml` (workspace root — already done in Task 1)
- Modify: `crates/velox/Cargo.toml`
- Modify: `crates/velox/src/lib.rs`

**Step 1: Add velox-ui to facade crate**

In `crates/velox/Cargo.toml`, add:
```toml
velox-ui = { workspace = true }
```

**Step 2: Update facade lib.rs**

Add to the existing re-exports:
```rust
pub use velox_ui as ui;
```

Add to the existing prelude:
```rust
pub use velox_ui::{
    auto, div, fr, pct, px, text, canvas, img, svg,
    Component, Element, IntoElement, InteractiveElement,
    ParentElement, Styled, Length, Style,
};
```

**Step 3: Full workspace verification**

Run: `cargo build --workspace`
Expected: Clean build

Run: `cargo test --workspace`
Expected: All tests pass (existing + new velox-ui tests)

Run: `cargo clippy --workspace`
Expected: No warnings

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(velox-ui): wire into workspace and facade prelude"
```

---

## Verification Checklist

After all tasks:

1. `cargo build --workspace` — clean
2. `cargo test --workspace` — all pass
3. `cargo clippy --workspace` — no warnings
4. Key API works end-to-end:
   - `div().flex_row().gap(px(8)).child(text("hello"))` compiles
   - Reconciler mounts element tree into NodeTree
   - Taffy computes layout, rects applied to nodes
   - Style diff distinguishes layout vs paint changes

## Future Tasks (Not In This Plan)

These are intentionally deferred to keep scope focused:

- **Cosmic-text integration in TextElement** — wire DrawGlyphs with actual font shaping
- **List element** — wrapping velox-list VirtualList (needs scroll wiring)
- **Input element** — wrapping velox-text EditableText
- **Overlay element** — wrapping velox-scene OverlayStack
- **Arena allocation** — swap Vec<AnyElement> for bumpalo arena (optimization)
- **Signal-driven re-render** — auto-subscribe Component to Signal dependencies
- **Keyed reconciliation** — hash-map based matching for list children
- **Hover/Active state tracking** — requires scene-level mouse tracking integration
- **Gradient backgrounds** — extend PaintCommand with gradient support
