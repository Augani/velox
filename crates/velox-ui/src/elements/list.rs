use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_list::VirtualList;
use velox_scene::Rect;

pub struct List {
    style: Style,
    item_height: f32,
    item_count: usize,
    item_builder: Box<dyn Fn(usize) -> AnyElement>,
}

pub fn list(
    item_height: f32,
    item_count: usize,
    builder: impl Fn(usize) -> AnyElement + 'static,
) -> List {
    List {
        style: Style::new(),
        item_height,
        item_count,
        item_builder: Box::new(builder),
    }
}

impl Styled for List {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for List {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

#[derive(Default)]
pub struct ListState {
    virtual_list: Option<VirtualList>,
}

impl Element for List {
    type State = ListState;

    fn layout(
        &mut self,
        state: &mut ListState,
        _children: &[AnyElement],
        _cx: &mut LayoutContext,
    ) -> LayoutRequest {
        if state.virtual_list.is_none() {
            state.virtual_list = Some(VirtualList::new(self.item_height, self.item_count));
        }
        if let Some(ref vl) = state.virtual_list {
            vl.set_item_count(self.item_count);
        }

        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(&mut self, state: &mut ListState, bounds: Rect, cx: &mut PaintContext) {
        let Some(ref virtual_list) = state.virtual_list else {
            return;
        };

        let viewport_height = bounds.height;
        let _offset = virtual_list.visible_range().map_or(0, |r| r.start_index);
        let count = self.item_count;

        let first_visible = {
            let scroll_offset = 0.0_f32;
            (scroll_offset / self.item_height).floor() as usize
        };
        let visible_count = (viewport_height / self.item_height).ceil() as usize + 1;
        let end_visible = (first_visible + visible_count).min(count);
        let start_visible = if let Some(range) = virtual_list.visible_range() {
            range.start_index
        } else {
            first_visible
        };
        let end = if let Some(range) = virtual_list.visible_range() {
            range.end_index
        } else {
            end_visible
        };

        cx.commands().push_clip(bounds);

        for i in start_visible..end {
            let mut child = (self.item_builder)(i);
            let item_y = bounds.y + (i as f32 * self.item_height)
                - (start_visible as f32 * self.item_height);
            let item_bounds = Rect::new(bounds.x, item_y, bounds.width, self.item_height);
            child.paint(item_bounds, cx);
        }

        cx.commands().pop_clip();
    }
}

impl List {
    pub fn scroll_by(&self, _state: &ListState, delta: f32) {
        if let Some(ref vl) = _state.virtual_list {
            vl.scroll_by(delta);
        }
    }

    pub fn scroll_to_index(&self, _state: &ListState, index: usize) {
        if let Some(ref vl) = _state.virtual_list {
            vl.scroll_to_index(index);
        }
    }
}

impl IntoElement for List {
    type Element = List;
    fn into_element(self) -> List {
        self
    }
}

impl IntoAnyElement for List {
    fn into_any_element(self) -> AnyElement {
        AnyElement::new(self, None, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::div;
    use crate::parent::IntoAnyElement;

    fn make_builder() -> impl Fn(usize) -> AnyElement + 'static {
        move |_i| div().into_any_element()
    }

    #[test]
    fn list_creates_virtual_list_on_layout() {
        let mut l = list(25.0, 10, make_builder());
        let mut state = ListState::default();
        let mut taffy = taffy::TaffyTree::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut cx = LayoutContext {
            taffy: &mut taffy,
            font_system: &mut font_system,
        };
        l.layout(&mut state, &[], &mut cx);
        assert!(state.virtual_list.is_some());
    }

    #[test]
    fn paint_emits_clip() {
        let mut l = list(25.0, 10, make_builder());
        let mut state = ListState::default();

        let mut taffy = taffy::TaffyTree::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut lcx = LayoutContext {
            taffy: &mut taffy,
            font_system: &mut font_system,
        };
        l.layout(&mut state, &[], &mut lcx);

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
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };

        l.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 100.0), &mut cx);

        let cmds = commands.commands();
        assert!(matches!(
            cmds.first(),
            Some(velox_scene::PaintCommand::PushClip(_))
        ));
        assert!(matches!(
            cmds.last(),
            Some(velox_scene::PaintCommand::PopClip)
        ));
    }

    #[test]
    fn builder_called_for_visible_items() {
        use std::cell::Cell;
        use std::rc::Rc;

        let call_count = Rc::new(Cell::new(0));
        let cc = call_count.clone();
        let builder = move |_i: usize| -> AnyElement {
            cc.set(cc.get() + 1);
            div().into_any_element()
        };

        let mut l = list(25.0, 100, builder);
        let mut state = ListState::default();

        let mut taffy = taffy::TaffyTree::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut lcx = LayoutContext {
            taffy: &mut taffy,
            font_system: &mut font_system,
        };
        l.layout(&mut state, &[], &mut lcx);

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
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };

        l.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 100.0), &mut cx);

        assert!(call_count.get() > 0);
        assert!(call_count.get() < 100);
    }
}
