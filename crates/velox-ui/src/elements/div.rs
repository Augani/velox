use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
use crate::interactive::{EventHandlers, InteractiveElement};
use crate::parent::{IntoAnyElement, ParentElement};
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{Point, Rect};

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

    fn paint(&mut self, _state: &mut DivState, bounds: Rect, cx: &mut PaintContext) {
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
