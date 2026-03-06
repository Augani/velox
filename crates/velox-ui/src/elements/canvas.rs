use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for Canvas {
    fn get_style(&self) -> &Style {
        &self.style
    }
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

    fn paint(&mut self, _state: &mut CanvasState, bounds: Rect, cx: &mut PaintContext) {
        (self.callback)(bounds, cx.commands());
    }
}

impl IntoElement for Canvas {
    type Element = Canvas;
    fn into_element(self) -> Canvas {
        self
    }
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
        let mut canvas_el = canvas(move |bounds, commands| {
            p.set(true);
            commands.fill_rect(bounds, velox_scene::Color::rgb(255, 0, 0));
        })
        .w(px(100.0))
        .h(px(100.0));

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
        };
        let mut state = CanvasState;
        canvas_el.paint(&mut state, Rect::new(0.0, 0.0, 100.0, 100.0), &mut cx);

        assert!(painted.get());
        assert_eq!(commands.commands().len(), 1);
    }
}
