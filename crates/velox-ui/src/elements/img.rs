use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
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
    #[allow(dead_code)]
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for Img {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

#[derive(Default)]
pub struct ImgState;

impl Element for Img {
    type State = ImgState;

    fn layout(
        &mut self,
        _: &mut ImgState,
        _: &[AnyElement],
        _: &mut LayoutContext,
    ) -> LayoutRequest {
        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(&mut self, _: &mut ImgState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }
    }
}

impl IntoElement for Img {
    type Element = Img;
    fn into_element(self) -> Img {
        self
    }
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
        let i = img(ImageSource::Path("photo.jpg".into())).object_fit(ObjectFit::Cover);
        assert_eq!(i.object_fit, ObjectFit::Cover);
    }
}
