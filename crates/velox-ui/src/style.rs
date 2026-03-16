use crate::length::Length;
use velox_scene::Color;

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
    Wait,
    Progress,
    Help,
    ZoomIn,
    ZoomOut,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
    ResizeEW,
    ResizeNS,
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

    pub background_gradient: Option<velox_scene::Gradient>,
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

        if other.background_gradient.is_some() {
            self.background_gradient = other.background_gradient.clone();
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
