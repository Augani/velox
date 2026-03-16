use std::any::TypeId;

use crate::interactive::EventHandlers;

pub type ElementKey = u64;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilityInfo {
    pub node: Option<velox_scene::AccessibilityNode>,
    pub text_content: Option<String>,
    pub text_runs: Vec<velox_scene::AccessibilityTextRun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessibilityAction {
    SetValue(String),
    ReplaceSelectedText(String),
    SetTextSelection(velox_scene::AccessibilityTextSelection),
}

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

    fn paint_after_children(
        &mut self,
        _state: &mut Self::State,
        _bounds: velox_scene::Rect,
        _cx: &mut PaintContext,
    ) {
    }

    fn accessibility(
        &mut self,
        _state: &mut Self::State,
        _children: &[AnyElement],
    ) -> AccessibilityInfo {
        AccessibilityInfo::default()
    }

    fn handle_accessibility_action(
        &mut self,
        _state: &mut Self::State,
        _action: &AccessibilityAction,
    ) -> bool {
        false
    }

    fn take_handlers(&mut self) -> EventHandlers {
        EventHandlers::default()
    }
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
    pub(crate) focused_node: Option<velox_scene::NodeId>,
    pub(crate) scroll_offset_x: f32,
    pub(crate) scroll_offset_y: f32,
    pub(crate) scale_factor: f32,
}

impl<'a> PaintContext<'a> {
    pub fn new(
        commands: &'a mut velox_scene::CommandList,
        theme: &'a velox_style::Theme,
        font_system: &'a mut velox_text::FontSystem,
        glyph_rasterizer: &'a mut velox_text::GlyphRasterizer,
    ) -> Self {
        Self {
            commands,
            theme,
            font_system,
            glyph_rasterizer,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        }
    }

    pub fn with_hovered(mut self, node: Option<velox_scene::NodeId>) -> Self {
        self.hovered_node = node;
        self
    }

    pub fn with_active(mut self, node: Option<velox_scene::NodeId>) -> Self {
        self.active_node = node;
        self
    }

    pub fn with_focused(mut self, node: Option<velox_scene::NodeId>) -> Self {
        self.focused_node = node;
        self
    }

    pub fn with_scale_factor(mut self, sf: f32) -> Self {
        self.scale_factor = sf;
        self
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

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

    pub fn is_focused(&self, node: velox_scene::NodeId) -> bool {
        self.focused_node == Some(node)
    }

    pub fn scroll_offset(&self) -> (f32, f32) {
        (self.scroll_offset_x, self.scroll_offset_y)
    }

    pub fn set_scroll_offset(&mut self, x: f32, y: f32) {
        self.scroll_offset_x = x;
        self.scroll_offset_y = y;
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
    fn paint_after_children_any(&mut self, bounds: velox_scene::Rect, cx: &mut PaintContext);
    fn accessibility_any(&mut self, children: &[AnyElement]) -> AccessibilityInfo;
    fn handle_accessibility_action_any(&mut self, action: &AccessibilityAction) -> bool;
    fn style(&self) -> &crate::style::Style;
    fn take_handlers(&mut self) -> EventHandlers;
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

    fn paint_after_children_any(&mut self, bounds: velox_scene::Rect, cx: &mut PaintContext) {
        self.element
            .paint_after_children(&mut self.state, bounds, cx);
    }

    fn accessibility_any(&mut self, children: &[AnyElement]) -> AccessibilityInfo {
        self.element.accessibility(&mut self.state, children)
    }

    fn handle_accessibility_action_any(&mut self, action: &AccessibilityAction) -> bool {
        self.element
            .handle_accessibility_action(&mut self.state, action)
    }

    fn style(&self) -> &crate::style::Style {
        self.element.get_style()
    }

    fn take_handlers(&mut self) -> EventHandlers {
        self.element.take_handlers()
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

    pub fn paint_after_children(&mut self, bounds: velox_scene::Rect, cx: &mut PaintContext) {
        self.element.paint_after_children_any(bounds, cx);
    }

    pub fn accessibility(&mut self) -> AccessibilityInfo {
        self.element.accessibility_any(&self.children)
    }

    pub fn handle_accessibility_action(&mut self, action: &AccessibilityAction) -> bool {
        self.element.handle_accessibility_action_any(action)
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

    pub fn take_handlers(&mut self) -> EventHandlers {
        self.element.take_handlers()
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
