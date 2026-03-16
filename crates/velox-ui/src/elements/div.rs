use crate::accessibility::{AccessibilityProps, AccessibleElement};
use crate::element::{
    AccessibilityInfo, AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest,
    PaintContext,
};
use crate::interactive::{EventHandlers, InteractiveElement};
use crate::parent::{IntoAnyElement, ParentElement};
use crate::scroll::ScrollbarColors;
use crate::style::{Overflow, Style};
use crate::styled::Styled;
use velox_scene::{Point, Rect};

pub struct Div {
    pub(crate) style: Style,
    pub(crate) hover_style: Option<Style>,
    pub(crate) active_style: Option<Style>,
    pub(crate) accessibility: AccessibilityProps,
    pub(crate) handlers: EventHandlers,
    pub(crate) children: Vec<AnyElement>,
}

pub fn div() -> Div {
    Div {
        style: Style::new(),
        hover_style: None,
        active_style: None,
        accessibility: AccessibilityProps::default(),
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.0
    }
}

impl Styled for Div {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl InteractiveElement for Div {
    fn handlers_mut(&mut self) -> &mut EventHandlers {
        &mut self.handlers
    }
}

impl AccessibleElement for Div {
    fn accessibility_props_mut(&mut self) -> &mut AccessibilityProps {
        &mut self.accessibility
    }
}

impl ParentElement for Div {
    fn children_mut(&mut self) -> &mut Vec<AnyElement> {
        &mut self.children
    }
}

impl HasStyle for Div {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

#[derive(Default)]
pub struct DivState {
    pub node_id: Option<velox_scene::NodeId>,
    pub scroll_offset_x: f32,
    pub scroll_offset_y: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub scroll_state: Option<crate::scroll::ScrollState>,
}

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

    fn take_handlers(&mut self) -> EventHandlers {
        std::mem::take(&mut self.handlers)
    }

    fn accessibility(
        &mut self,
        _state: &mut DivState,
        _children: &[AnyElement],
    ) -> AccessibilityInfo {
        if self.accessibility.is_empty() {
            AccessibilityInfo::default()
        } else {
            let mut node = self.accessibility.resolve(
                velox_scene::AccessibilityRole::Group,
                None,
                None,
                false,
            );
            if self.handlers.focusable || self.handlers.on_focus.is_some() {
                node = node.supports_focus_actions();
            }
            if self.handlers.on_click.is_some() {
                node = node.supports_click_action();
            }
            AccessibilityInfo {
                node: Some(node),
                text_content: None,
                text_runs: Vec::new(),
            }
        }
    }

    fn paint(&mut self, state: &mut DivState, bounds: Rect, cx: &mut PaintContext) {
        let mut effective = self.style.clone();
        if let Some(node) = state.node_id {
            if cx.is_hovered(node)
                && let Some(hover) = &self.hover_style
            {
                effective.merge(hover);
            }
            if cx.is_active(node)
                && let Some(active) = &self.active_style
            {
                effective.merge(active);
            }
        }

        let corner_radius = effective.border_radius_tl.unwrap_or(0.0);

        if let Some(ref gradient) = effective.background_gradient {
            cx.commands().fill_gradient(bounds, gradient.clone());
        } else if let Some(bg) = effective.background {
            if corner_radius > 0.0 {
                cx.commands().fill_rounded_rect(bounds, bg, corner_radius);
            } else {
                cx.commands().fill_rect(bounds, bg);
            }
        }

        if let Some(bc) = effective.border_color {
            let bt = effective.border_top_width.unwrap_or(0.0);
            let br = effective.border_right_width.unwrap_or(0.0);
            let bb = effective.border_bottom_width.unwrap_or(0.0);
            let bl = effective.border_left_width.unwrap_or(0.0);

            if bt == br && br == bb && bb == bl && bt > 0.0 {
                cx.commands().stroke_rect(bounds, bc, bt);
            } else {
                if bt > 0.0 {
                    cx.commands()
                        .fill_rect(Rect::new(bounds.x, bounds.y, bounds.width, bt), bc);
                }
                if bb > 0.0 {
                    cx.commands().fill_rect(
                        Rect::new(bounds.x, bounds.y + bounds.height - bb, bounds.width, bb),
                        bc,
                    );
                }
                if bl > 0.0 {
                    cx.commands()
                        .fill_rect(Rect::new(bounds.x, bounds.y, bl, bounds.height), bc);
                }
                if br > 0.0 {
                    cx.commands().fill_rect(
                        Rect::new(bounds.x + bounds.width - br, bounds.y, br, bounds.height),
                        bc,
                    );
                }
            }
        }
        for shadow in &effective.box_shadows {
            cx.commands().box_shadow(
                bounds,
                shadow.color,
                shadow.blur_radius,
                Point::new(shadow.offset_x, shadow.offset_y),
                shadow.spread,
            );
        }

        let is_scrollable = effective.overflow_x == Some(Overflow::Scroll)
            || effective.overflow_y == Some(Overflow::Scroll);

        if is_scrollable {
            cx.commands().push_clip(bounds);
            cx.set_scroll_offset(state.scroll_offset_x, state.scroll_offset_y);
        }
    }

    fn paint_after_children(&mut self, state: &mut DivState, bounds: Rect, cx: &mut PaintContext) {
        let is_scrollable = self.style.overflow_x == Some(Overflow::Scroll)
            || self.style.overflow_y == Some(Overflow::Scroll);

        if is_scrollable {
            cx.commands().pop_clip();
            cx.set_scroll_offset(0.0, 0.0);

            let axis = match (
                self.style.overflow_x == Some(Overflow::Scroll),
                self.style.overflow_y == Some(Overflow::Scroll),
            ) {
                (true, true) => crate::scroll::ScrollAxis::Both,
                (true, false) => crate::scroll::ScrollAxis::Horizontal,
                _ => crate::scroll::ScrollAxis::Vertical,
            };
            let scroll_state = state
                .scroll_state
                .get_or_insert_with(|| crate::scroll::ScrollState::new(axis));
            scroll_state.set_viewport_size(bounds.width, bounds.height);
            scroll_state.set_content_size(state.content_width, state.content_height);
            scroll_state.scroll_to(state.scroll_offset_x, state.scroll_offset_y, false);

            let colors = ScrollbarColors::default();
            scroll_state.paint_scrollbars(cx.commands(), bounds, &colors);
        }

        if let Some(node) = state.node_id
            && cx.is_focused(node) {
                let accent = cx.theme().palette.accent;
                let focus_color = velox_scene::Color::rgba(accent.r, accent.g, accent.b, accent.a);
                let offset = 2.0;
                let stroke_width = 2.0;
                let ring = Rect::new(
                    bounds.x - offset,
                    bounds.y - offset,
                    bounds.width + offset * 2.0,
                    bounds.height + offset * 2.0,
                );
                cx.commands().stroke_rect(ring, focus_color, stroke_width);
            }
    }
}

impl IntoElement for Div {
    type Element = Div;
    fn into_element(self) -> Div {
        self
    }
}

impl IntoAnyElement for Div {
    fn into_any_element(self) -> AnyElement {
        let mut d = self;
        let children = std::mem::take(&mut d.children);
        AnyElement::new(d, None, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::length::px;
    use velox_scene::Color;

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
        assert_eq!(
            d.hover_style.unwrap().background,
            Some(Color::rgb(50, 50, 50))
        );
    }

    #[test]
    fn div_with_event_handler() {
        let d = div().on_click(|_| {}).cursor_pointer();

        assert!(d.handlers.on_click.is_some());
        assert_eq!(d.style.cursor, Some(crate::style::CursorStyle::Pointer));
    }

    #[test]
    fn div_paint_emits_fill_rect() {
        let mut d = div().bg(Color::rgb(255, 0, 0));
        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut glyph_rasterizer = velox_text::GlyphRasterizer::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut font_system,
            glyph_rasterizer: &mut glyph_rasterizer,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        let mut state = DivState::default();
        d.paint(&mut state, bounds, &mut cx);

        assert_eq!(commands.commands().len(), 1);
        assert!(matches!(
            commands.commands()[0],
            velox_scene::PaintCommand::FillRect { .. }
        ));
    }

    fn make_paint_ctx<'a>(
        commands: &'a mut velox_scene::CommandList,
        theme: &'a velox_style::Theme,
        font_system: &'a mut velox_text::FontSystem,
        glyph_rasterizer: &'a mut velox_text::GlyphRasterizer,
        hovered: Option<velox_scene::NodeId>,
        active: Option<velox_scene::NodeId>,
    ) -> PaintContext<'a> {
        PaintContext {
            commands,
            theme,
            font_system,
            glyph_rasterizer,
            hovered_node: hovered,
            active_node: active,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        }
    }

    #[test]
    fn hover_style_applied_when_hovered() {
        let mut d = div()
            .bg(Color::rgb(0, 0, 0))
            .hover(|s| s.bg(Color::rgb(255, 0, 0)));

        let mut tree = velox_scene::NodeTree::new();
        let node_id = tree.insert(None);
        let mut state = DivState {
            node_id: Some(node_id),
            ..Default::default()
        };

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, Some(node_id), None);

        d.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 50.0), &mut cx);

        match &commands.commands()[0] {
            velox_scene::PaintCommand::FillRect { color, .. } => {
                assert_eq!(*color, Color::rgb(255, 0, 0));
            }
            _ => panic!("expected FillRect with hover color"),
        }
    }

    #[test]
    fn base_style_when_not_hovered() {
        let mut d = div()
            .bg(Color::rgb(0, 0, 0))
            .hover(|s| s.bg(Color::rgb(255, 0, 0)));

        let mut tree = velox_scene::NodeTree::new();
        let node_id = tree.insert(None);
        let mut state = DivState {
            node_id: Some(node_id),
            ..Default::default()
        };

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);

        d.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 50.0), &mut cx);

        match &commands.commands()[0] {
            velox_scene::PaintCommand::FillRect { color, .. } => {
                assert_eq!(*color, Color::rgb(0, 0, 0));
            }
            _ => panic!("expected FillRect with base color"),
        }
    }

    #[test]
    fn active_overrides_hover() {
        let mut d = div()
            .bg(Color::rgb(0, 0, 0))
            .hover(|s| s.bg(Color::rgb(100, 100, 100)))
            .active(|s| s.bg(Color::rgb(255, 0, 0)));

        let mut tree = velox_scene::NodeTree::new();
        let node_id = tree.insert(None);
        let mut state = DivState {
            node_id: Some(node_id),
            ..Default::default()
        };

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(
            &mut commands,
            &theme,
            &mut fs,
            &mut gr,
            Some(node_id),
            Some(node_id),
        );

        d.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 50.0), &mut cx);

        match &commands.commands()[0] {
            velox_scene::PaintCommand::FillRect { color, .. } => {
                assert_eq!(*color, Color::rgb(255, 0, 0));
            }
            _ => panic!("expected FillRect with active color"),
        }
    }

    #[test]
    fn no_hover_style_is_noop() {
        let mut d = div().bg(Color::rgb(0, 0, 0));

        let mut tree = velox_scene::NodeTree::new();
        let node_id = tree.insert(None);
        let mut state = DivState {
            node_id: Some(node_id),
            ..Default::default()
        };

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, Some(node_id), None);

        d.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 50.0), &mut cx);

        match &commands.commands()[0] {
            velox_scene::PaintCommand::FillRect { color, .. } => {
                assert_eq!(*color, Color::rgb(0, 0, 0));
            }
            _ => panic!("expected FillRect with base color"),
        }
    }

    #[test]
    fn gradient_emits_fill_gradient() {
        let mut d = div().bg_linear_gradient(
            90.0,
            vec![(0.0, Color::rgb(255, 0, 0)), (1.0, Color::rgb(0, 0, 255))],
        );

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);
        let mut state = DivState::default();

        d.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 50.0), &mut cx);

        assert!(matches!(
            commands.commands()[0],
            velox_scene::PaintCommand::FillGradient { .. }
        ));
    }

    #[test]
    fn gradient_style_merge_preserves() {
        let mut base = crate::style::Style::new();
        base.background_gradient = Some(velox_scene::Gradient::Linear {
            angle_deg: 45.0,
            stops: vec![
                velox_scene::GradientStop {
                    offset: 0.0,
                    color: Color::rgb(255, 0, 0),
                },
                velox_scene::GradientStop {
                    offset: 1.0,
                    color: Color::rgb(0, 255, 0),
                },
            ],
        });

        let overlay = crate::style::Style::new();
        base.merge(&overlay);
        assert!(base.background_gradient.is_some());
    }

    #[test]
    fn scroll_div_emits_push_clip() {
        let mut d = div().overflow_y_scroll().bg(Color::rgb(240, 240, 240));

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);
        let mut state = DivState::default();

        d.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 400.0), &mut cx);

        let has_clip = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::PushClip(_)));
        assert!(has_clip, "scrollable div should emit PushClip");
    }

    #[test]
    fn scroll_div_paint_after_children_pops_clip() {
        let mut d = div().overflow_y_scroll().bg(Color::rgb(240, 240, 240));

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);
        let mut state = DivState::default();

        d.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 400.0), &mut cx);
        d.paint_after_children(&mut state, Rect::new(0.0, 0.0, 200.0, 400.0), &mut cx);

        let has_pop = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::PopClip));
        assert!(has_pop, "paint_after_children should emit PopClip");
    }

    #[test]
    fn non_scroll_div_no_clip() {
        let mut d = div().bg(Color::rgb(240, 240, 240));

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);
        let mut state = DivState::default();

        d.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 400.0), &mut cx);

        let has_clip = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::PushClip(_)));
        assert!(!has_clip, "non-scrollable div should not emit PushClip");
    }

    #[test]
    fn scroll_div_sets_scroll_offset() {
        let mut d = div().overflow_y_scroll();

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);
        let mut state = DivState {
            scroll_offset_y: 50.0,
            ..Default::default()
        };

        d.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 400.0), &mut cx);

        let (_, offset_y) = cx.scroll_offset();
        assert_eq!(offset_y, 50.0);
    }

    #[test]
    fn focus_ring_emitted_when_focused() {
        let mut d = div().bg(Color::rgb(0, 0, 0));

        let mut tree = velox_scene::NodeTree::new();
        let node_id = tree.insert(None);
        let mut state = DivState {
            node_id: Some(node_id),
            ..Default::default()
        };

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut fs,
            glyph_rasterizer: &mut gr,
            hovered_node: None,
            active_node: None,
            focused_node: Some(node_id),
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };

        let bounds = Rect::new(10.0, 10.0, 100.0, 50.0);
        d.paint(&mut state, bounds, &mut cx);
        d.paint_after_children(&mut state, bounds, &mut cx);

        let has_stroke = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::StrokeRect { .. }));
        assert!(
            has_stroke,
            "focused div should emit StrokeRect for focus ring"
        );
    }

    #[test]
    fn no_focus_ring_when_not_focused() {
        let mut d = div().bg(Color::rgb(0, 0, 0));

        let mut tree = velox_scene::NodeTree::new();
        let node_id = tree.insert(None);
        let mut state = DivState {
            node_id: Some(node_id),
            ..Default::default()
        };

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = make_paint_ctx(&mut commands, &theme, &mut fs, &mut gr, None, None);

        let bounds = Rect::new(10.0, 10.0, 100.0, 50.0);
        d.paint(&mut state, bounds, &mut cx);
        d.paint_after_children(&mut state, bounds, &mut cx);

        let has_stroke = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::StrokeRect { .. }));
        assert!(!has_stroke, "unfocused div should not emit StrokeRect");
    }
}
