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

    fn paint(&mut self, state: &mut Self::State, bounds: velox_scene::Rect, cx: &mut PaintContext);
}

pub struct LayoutRequest {
    pub taffy_style: taffy::Style,
}

pub struct LayoutContext<'a> {
    #[allow(dead_code)]
    pub(crate) taffy: &'a mut taffy::TaffyTree<()>,
    pub(crate) font_system: &'a mut velox_text::FontSystem,
}

impl<'a> LayoutContext<'a> {
    pub fn font_system(&mut self) -> &mut velox_text::FontSystem {
        self.font_system
    }
}

pub struct PaintContext<'a> {
    pub(crate) commands: &'a mut velox_scene::CommandList,
    pub(crate) theme: &'a velox_style::Theme,
    pub(crate) font_system: &'a mut velox_text::FontSystem,
    pub(crate) glyph_rasterizer: &'a mut velox_text::GlyphRasterizer,
    pub(crate) hovered_node: Option<velox_scene::NodeId>,
    pub(crate) active_node: Option<velox_scene::NodeId>,
}

impl<'a> PaintContext<'a> {
    pub fn commands(&mut self) -> &mut velox_scene::CommandList {
        self.commands
    }

    pub fn theme(&self) -> &velox_style::Theme {
        self.theme
    }

    pub fn font_system(&mut self) -> &mut velox_text::FontSystem {
        self.font_system
    }

    pub fn glyph_rasterizer(&mut self) -> &mut velox_text::GlyphRasterizer {
        self.glyph_rasterizer
    }

    pub fn is_hovered(&self, node: velox_scene::NodeId) -> bool {
        self.hovered_node == Some(node)
    }

    pub fn is_active(&self, node: velox_scene::NodeId) -> bool {
        self.active_node == Some(node)
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub(crate) key: Option<ElementKey>,
    pub(crate) children: Vec<AnyElement>,
}

trait AnyElementTrait: 'static {
    fn element_type_id(&self) -> TypeId;
    fn layout_any(&mut self, children: &[AnyElement], cx: &mut LayoutContext) -> LayoutRequest;
    fn paint_any(&mut self, bounds: velox_scene::Rect, cx: &mut PaintContext);
    fn style(&self) -> &crate::style::Style;
}

struct TypedElement<E: Element> {
    element: E,
    state: E::State,
}

impl<E: Element + HasStyle> AnyElementTrait for TypedElement<E> {
    fn element_type_id(&self) -> TypeId {
        self.element.element_type_id()
    }

    fn layout_any(&mut self, children: &[AnyElement], cx: &mut LayoutContext) -> LayoutRequest {
        self.element.layout(&mut self.state, children, cx)
    }

    fn paint_any(&mut self, bounds: velox_scene::Rect, cx: &mut PaintContext) {
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
    pub fn new<E: Element + HasStyle>(
        element: E,
        key: Option<ElementKey>,
        children: Vec<AnyElement>,
    ) -> Self {
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
            fn layout(
                &mut self,
                _: &mut (),
                _: &[AnyElement],
                _: &mut LayoutContext,
            ) -> LayoutRequest {
                LayoutRequest {
                    taffy_style: taffy::Style::default(),
                }
            }
            fn paint(&mut self, _: &mut (), _: velox_scene::Rect, _: &mut PaintContext) {}
        }
        impl IntoElement for DummyElement {
            type Element = DummyEl;
            fn into_element(self) -> DummyEl {
                DummyEl
            }
        }

        let keyed = DummyElement.key(42);
        assert_eq!(keyed.key, 42);
    }
}
