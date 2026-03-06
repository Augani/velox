use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for TextElement {
    fn get_style(&self) -> &Style {
        &self.style
    }
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

    fn paint(&mut self, _state: &mut TextState, bounds: Rect, cx: &mut PaintContext) {
        let color = self
            .style
            .text_color
            .unwrap_or(velox_scene::Color::rgb(0, 0, 0));
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }
        cx.commands().fill_rect(
            Rect::new(
                bounds.x,
                bounds.y,
                bounds.width.min(self.content.len() as f32 * 8.0),
                bounds.height.min(16.0),
            ),
            color,
        );
    }
}

impl IntoElement for TextElement {
    type Element = TextElement;
    fn into_element(self) -> TextElement {
        self
    }
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
