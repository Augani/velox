use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::Rect;

pub struct Svg {
    #[allow(dead_code)]
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for Svg {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

#[derive(Default)]
pub struct SvgState;

impl Element for Svg {
    type State = SvgState;

    fn layout(
        &mut self,
        _: &mut SvgState,
        _: &[AnyElement],
        _: &mut LayoutContext,
    ) -> LayoutRequest {
        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(&mut self, _: &mut SvgState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(color) = self.style.text_color {
            cx.commands().fill_rect(bounds, color);
        }
    }
}

impl IntoElement for Svg {
    type Element = Svg;
    fn into_element(self) -> Svg {
        self
    }
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
        assert_eq!(
            s.style.text_color,
            Some(velox_scene::Color::rgb(100, 100, 100))
        );
    }
}
