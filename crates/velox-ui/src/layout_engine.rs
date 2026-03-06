use crate::length::Length;
use crate::style::{self, AlignItems, Display, FlexDirection, FlexWrap, JustifyContent, Position};
use taffy::prelude::*;
use taffy::TaffyResult;

pub struct LayoutEngine {
    pub(crate) taffy: TaffyTree<()>,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    pub fn new_leaf(&mut self, style: &self::style::Style) -> TaffyResult<NodeId> {
        let ts = convert_style(style);
        self.taffy.new_leaf(ts)
    }

    pub fn new_with_children(
        &mut self,
        style: &self::style::Style,
        children: &[NodeId],
    ) -> TaffyResult<NodeId> {
        let ts = convert_style(style);
        self.taffy.new_with_children(ts, children)
    }

    pub fn set_style(&mut self, node: NodeId, style: &self::style::Style) -> TaffyResult<()> {
        let ts = convert_style(style);
        self.taffy.set_style(node, ts)
    }

    pub fn remove(&mut self, node: NodeId) -> TaffyResult<NodeId> {
        self.taffy.remove(node)
    }

    pub fn compute_layout(
        &mut self,
        root: NodeId,
        available: Size<AvailableSpace>,
    ) -> TaffyResult<()> {
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

pub fn convert_style(style: &self::style::Style) -> taffy::Style {
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

    if let Some(v) = &style.width {
        ts.size.width = convert_length_dimension(v);
    }
    if let Some(v) = &style.height {
        ts.size.height = convert_length_dimension(v);
    }
    if let Some(v) = &style.min_width {
        ts.min_size.width = convert_length_dimension(v);
    }
    if let Some(v) = &style.min_height {
        ts.min_size.height = convert_length_dimension(v);
    }
    if let Some(v) = &style.max_width {
        ts.max_size.width = convert_length_dimension(v);
    }
    if let Some(v) = &style.max_height {
        ts.max_size.height = convert_length_dimension(v);
    }
    if let Some(v) = &style.aspect_ratio {
        ts.aspect_ratio = Some(*v);
    }

    if let Some(v) = &style.padding_top {
        ts.padding.top = convert_length_lp(v);
    }
    if let Some(v) = &style.padding_right {
        ts.padding.right = convert_length_lp(v);
    }
    if let Some(v) = &style.padding_bottom {
        ts.padding.bottom = convert_length_lp(v);
    }
    if let Some(v) = &style.padding_left {
        ts.padding.left = convert_length_lp(v);
    }

    if let Some(v) = &style.margin_top {
        ts.margin.top = convert_length(v);
    }
    if let Some(v) = &style.margin_right {
        ts.margin.right = convert_length(v);
    }
    if let Some(v) = &style.margin_bottom {
        ts.margin.bottom = convert_length(v);
    }
    if let Some(v) = &style.margin_left {
        ts.margin.left = convert_length(v);
    }

    if let Some(v) = &style.inset_top {
        ts.inset.top = convert_length(v);
    }
    if let Some(v) = &style.inset_right {
        ts.inset.right = convert_length(v);
    }
    if let Some(v) = &style.inset_bottom {
        ts.inset.bottom = convert_length(v);
    }
    if let Some(v) = &style.inset_left {
        ts.inset.left = convert_length(v);
    }

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
    if let Some(v) = style.flex_grow {
        ts.flex_grow = v;
    }
    if let Some(v) = style.flex_shrink {
        ts.flex_shrink = v;
    }
    if let Some(v) = &style.flex_basis {
        ts.flex_basis = convert_length_dimension(v);
    }

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
        ts.gap = Size {
            width: g,
            height: g,
        };
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
        let root = engine
            .new_with_children(&parent_style, &[child1, child2])
            .unwrap();

        engine
            .compute_layout(
                root,
                Size {
                    width: AvailableSpace::Definite(200.0),
                    height: AvailableSpace::Definite(100.0),
                },
            )
            .unwrap();

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
        let root = engine
            .new_with_children(&parent_style, &[fixed, flexible])
            .unwrap();

        engine
            .compute_layout(
                root,
                Size {
                    width: AvailableSpace::Definite(300.0),
                    height: AvailableSpace::Definite(100.0),
                },
            )
            .unwrap();

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

        engine
            .compute_layout(
                root,
                Size {
                    width: AvailableSpace::Definite(200.0),
                    height: AvailableSpace::Definite(200.0),
                },
            )
            .unwrap();

        let layout = engine.layout(child).unwrap();
        assert_eq!(layout.location.x, 20.0);
        assert_eq!(layout.location.y, 10.0);
    }
}
