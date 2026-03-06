use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
use crate::parent::{IntoAnyElement, ParentElement};
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{ModalConfig, OverlayId, OverlayStack, Rect};

pub struct Overlay {
    style: Style,
    pub modal_config: Option<ModalConfig>,
    children: Vec<AnyElement>,
}

pub fn overlay() -> Overlay {
    Overlay {
        style: Style::new(),
        modal_config: None,
        children: Vec::new(),
    }
}

pub fn modal(config: ModalConfig) -> Overlay {
    Overlay {
        style: Style::new(),
        modal_config: Some(config),
        children: Vec::new(),
    }
}

impl Styled for Overlay {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for Overlay {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

impl ParentElement for Overlay {
    fn children_mut(&mut self) -> &mut Vec<AnyElement> {
        &mut self.children
    }
}

#[derive(Default)]
pub struct OverlayState {
    overlay_id: Option<OverlayId>,
    overlay_stack: Option<*mut OverlayStack>,
}

impl OverlayState {
    pub fn overlay_id(&self) -> Option<OverlayId> {
        self.overlay_id
    }

    pub fn remove_overlay(&mut self) {
        if let (Some(id), Some(stack_ptr)) = (self.overlay_id.take(), self.overlay_stack.take()) {
            let stack = unsafe { &mut *stack_ptr };
            stack.pop_overlay(id);
        }
    }
}

impl Element for Overlay {
    type State = OverlayState;

    fn layout(
        &mut self,
        _state: &mut OverlayState,
        _children: &[AnyElement],
        _cx: &mut LayoutContext,
    ) -> LayoutRequest {
        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(&mut self, _state: &mut OverlayState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }

        for child in &mut self.children {
            child.paint(bounds, cx);
        }
    }
}

impl IntoElement for Overlay {
    type Element = Overlay;
    fn into_element(self) -> Overlay {
        self
    }
}

impl IntoAnyElement for Overlay {
    fn into_any_element(self) -> AnyElement {
        let mut o = self;
        let children = std::mem::take(&mut o.children);
        AnyElement::new(o, None, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::div;
    use crate::parent::IntoAnyElement as _;

    #[test]
    fn overlay_creates_with_children() {
        let o = overlay().child(div()).child(div());
        assert_eq!(o.children.len(), 2);
    }

    #[test]
    fn modal_stores_config() {
        let config = ModalConfig {
            backdrop_dismisses: true,
            trap_focus: true,
            blocks_parent: true,
        };
        let o = modal(config);
        assert!(o.modal_config.is_some());
        let c = o.modal_config.unwrap();
        assert!(c.backdrop_dismisses);
        assert!(c.trap_focus);
    }

    #[test]
    fn overlay_state_default_is_none() {
        let state = OverlayState::default();
        assert!(state.overlay_id().is_none());
    }

    #[test]
    fn overlay_paints_children() {
        let mut o = overlay()
            .bg(velox_scene::Color::rgba(0, 0, 0, 128))
            .child(div().bg(velox_scene::Color::rgb(255, 255, 255)));

        let mut state = OverlayState::default();
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
        };

        o.paint(&mut state, Rect::new(0.0, 0.0, 400.0, 300.0), &mut cx);

        assert!(commands.commands().len() >= 1);
    }
}
