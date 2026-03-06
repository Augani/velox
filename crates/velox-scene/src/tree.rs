use slotmap::SlotMap;

use crate::event::KeyEvent;
use crate::event_handler::{EventContext, EventHandler};
use crate::geometry::{Point, Rect};
use crate::layout::Layout;
use crate::node::NodeId;
use crate::paint::CommandList;
use crate::painter::Painter;

pub(crate) struct NodeData {
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
    pub(crate) rect: Rect,
    pub(crate) visible: bool,
    pub(crate) layout_dirty: bool,
    pub(crate) paint_dirty: bool,
    pub(crate) hit_test_transparent: bool,
    pub(crate) painter: Option<Box<dyn Painter>>,
    pub(crate) layout: Option<Box<dyn Layout>>,
    pub(crate) event_handler: Option<Box<dyn EventHandler>>,
}

impl NodeData {
    fn new(parent: Option<NodeId>) -> Self {
        Self {
            parent,
            children: Vec::new(),
            rect: Rect::zero(),
            visible: true,
            layout_dirty: true,
            paint_dirty: true,
            hit_test_transparent: false,
            painter: None,
            layout: None,
            event_handler: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct EventDispatchResult {
    pub consumed: bool,
    pub redraw_requested: bool,
    pub clipboard_write: Option<String>,
}

#[derive(Default)]
pub struct NodeTree {
    nodes: SlotMap<NodeId, NodeData>,
    root: Option<NodeId>,
}

impl NodeTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.insert(NodeData::new(parent));

        match parent {
            Some(parent_id) => {
                if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                    parent_node.children.push(id);
                }
            }
            None => {
                if self.root.is_none() {
                    self.root = Some(id);
                }
            }
        }

        id
    }

    pub fn remove(&mut self, id: NodeId) {
        let Some(node) = self.nodes.get(id) else {
            return;
        };

        let parent = node.parent;
        let children: Vec<NodeId> = node.children.clone();

        for child in children {
            self.remove(child);
        }

        if let Some(parent_id) = parent {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.children.retain(|&c| c != id);
            }
        }

        if self.root == Some(id) {
            self.root = None;
        }

        self.nodes.remove(id);
    }

    pub fn reparent(&mut self, id: NodeId, new_parent: NodeId) {
        if !self.nodes.contains_key(id) || !self.nodes.contains_key(new_parent) {
            return;
        }

        if let Some(old_parent) = self.nodes.get(id).and_then(|n| n.parent) {
            if let Some(parent_node) = self.nodes.get_mut(old_parent) {
                parent_node.children.retain(|&c| c != id);
            }
        }

        if let Some(node) = self.nodes.get_mut(id) {
            node.parent = Some(new_parent);
        }

        if let Some(parent_node) = self.nodes.get_mut(new_parent) {
            parent_node.children.push(id);
        }
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(id).and_then(|n| n.parent)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes
            .get(id)
            .map(|n| n.children.as_slice())
            .unwrap_or(&[])
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id)
    }

    pub fn set_rect(&mut self, id: NodeId, rect: Rect) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.rect = rect;
            node.paint_dirty = true;
        }
    }

    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.nodes.get(id).map(|n| n.rect)
    }

    pub fn set_visible(&mut self, id: NodeId, visible: bool) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.visible = visible;
            node.paint_dirty = true;
        }
    }

    pub fn is_visible(&self, id: NodeId) -> Option<bool> {
        self.nodes.get(id).map(|n| n.visible)
    }

    pub fn set_hit_test_transparent(&mut self, id: NodeId, transparent: bool) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.hit_test_transparent = transparent;
        }
    }

    pub fn is_hit_test_transparent(&self, id: NodeId) -> Option<bool> {
        self.nodes.get(id).map(|n| n.hit_test_transparent)
    }

    pub fn mark_layout_dirty(&mut self, id: NodeId) {
        let mut current = Some(id);
        while let Some(cid) = current {
            match self.nodes.get_mut(cid) {
                Some(node) if node.layout_dirty => break,
                Some(node) => {
                    node.layout_dirty = true;
                    current = node.parent;
                }
                None => break,
            }
        }
    }

    pub fn mark_paint_dirty(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.paint_dirty = true;
        }
    }

    pub fn clear_dirty(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.layout_dirty = false;
            node.paint_dirty = false;
        }
    }

    pub fn set_painter(&mut self, id: NodeId, painter: impl Painter + 'static) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.painter = Some(Box::new(painter));
            node.paint_dirty = true;
        }
    }

    pub fn run_paint(&mut self, commands: &mut CommandList) {
        let Some(root) = self.root else { return };
        self.paint_node(root, commands);
    }

    fn paint_node(&mut self, id: NodeId, commands: &mut CommandList) {
        let Some(data) = self.nodes.get(id) else {
            return;
        };
        if !data.visible {
            return;
        }

        let rect = data.rect;
        let children = data.children.clone();

        commands.push_clip(rect);

        let painter = self.nodes.get_mut(id).and_then(|d| d.painter.take());
        if let Some(ref p) = painter {
            p.paint(rect, commands);
        }
        if let Some(data) = self.nodes.get_mut(id) {
            data.painter = painter;
        }

        for child in children {
            self.paint_node(child, commands);
        }

        commands.pop_clip();

        if let Some(data) = self.nodes.get_mut(id) {
            data.paint_dirty = false;
        }
    }

    pub fn set_layout(&mut self, id: NodeId, layout: impl Layout + 'static) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.layout = Some(Box::new(layout));
            node.layout_dirty = true;
        }
    }

    pub fn run_layout(&mut self) {
        let Some(root) = self.root else { return };
        self.layout_node(root);
    }

    fn layout_node(&mut self, id: NodeId) {
        let Some(data) = self.nodes.get(id) else {
            return;
        };

        let is_dirty = data.layout_dirty;
        let has_layout = data.layout.is_some();
        let rect = data.rect;
        let children = data.children.clone();

        if is_dirty && has_layout {
            let layout = self.nodes.get_mut(id).and_then(|d| d.layout.take());
            if let Some(ref l) = layout {
                l.compute(rect, &children, self);
            }
            if let Some(data) = self.nodes.get_mut(id) {
                data.layout = layout;
            }
        }

        if let Some(data) = self.nodes.get_mut(id) {
            data.layout_dirty = false;
        }

        for child in children {
            self.layout_node(child);
        }
    }

    pub fn hit_test(&self, point: Point) -> Option<NodeId> {
        let root = self.root?;
        self.hit_test_node(root, point)
    }

    pub fn set_event_handler(&mut self, id: NodeId, handler: impl EventHandler + 'static) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.event_handler = Some(Box::new(handler));
        }
    }

    pub fn dispatch_key_event(&mut self, id: NodeId, event: &KeyEvent) -> bool {
        self.dispatch_key_event_with_context(id, event, None).consumed
    }

    pub fn dispatch_key_event_with_context(
        &mut self,
        id: NodeId,
        event: &KeyEvent,
        clipboard_read: Option<String>,
    ) -> EventDispatchResult {
        let Some(data) = self.nodes.get(id) else {
            return EventDispatchResult::default();
        };
        let rect = data.rect;
        let handler = self.nodes.get_mut(id).and_then(|d| d.event_handler.take());
        if let Some(mut h) = handler {
            let mut ctx = EventContext::new(rect);
            ctx.set_clipboard_content(clipboard_read);
            let consumed = h.handle_key(event, &mut ctx);
            let redraw_requested = ctx.redraw_requested();
            let clipboard_write = ctx.take_clipboard_write();
            if let Some(data) = self.nodes.get_mut(id) {
                data.event_handler = Some(h);
            }
            EventDispatchResult {
                consumed,
                redraw_requested,
                clipboard_write,
            }
        } else {
            EventDispatchResult::default()
        }
    }

    pub fn dispatch_mouse_event(&mut self, id: NodeId, event: &crate::event::MouseEvent) -> bool {
        self.dispatch_mouse_event_with_context(id, event).consumed
    }

    pub fn dispatch_mouse_event_with_context(
        &mut self,
        id: NodeId,
        event: &crate::event::MouseEvent,
    ) -> EventDispatchResult {
        let Some(data) = self.nodes.get(id) else {
            return EventDispatchResult::default();
        };
        let rect = data.rect;
        let handler = self.nodes.get_mut(id).and_then(|d| d.event_handler.take());
        if let Some(mut h) = handler {
            let mut ctx = EventContext::new(rect);
            let consumed = h.handle_mouse(event, &mut ctx);
            let redraw_requested = ctx.redraw_requested();
            let clipboard_write = ctx.take_clipboard_write();
            if let Some(data) = self.nodes.get_mut(id) {
                data.event_handler = Some(h);
            }
            EventDispatchResult {
                consumed,
                redraw_requested,
                clipboard_write,
            }
        } else {
            EventDispatchResult::default()
        }
    }

    fn hit_test_node(&self, id: NodeId, point: Point) -> Option<NodeId> {
        let data = self.nodes.get(id)?;

        if !data.visible || data.hit_test_transparent {
            return None;
        }

        if !data.rect.contains(point) {
            return None;
        }

        for &child in data.children.iter().rev() {
            if let Some(hit) = self.hit_test_node(child, point) {
                return Some(hit);
            }
        }

        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_root_node() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);

        assert_eq!(tree.root(), Some(root));
        assert_eq!(tree.parent(root), None);
    }

    #[test]
    fn insert_child_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child1 = tree.insert(Some(root));
        let child2 = tree.insert(Some(root));

        assert_eq!(tree.children(root), &[child1, child2]);
        assert_eq!(tree.parent(child1), Some(root));
        assert_eq!(tree.parent(child2), Some(root));
    }

    #[test]
    fn remove_node_and_descendants() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));
        let grandchild = tree.insert(Some(child));

        tree.remove(child);

        assert!(!tree.contains(child));
        assert!(!tree.contains(grandchild));
        assert!(tree.children(root).is_empty());
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn reparent_node() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let parent_a = tree.insert(Some(root));
        let parent_b = tree.insert(Some(root));
        let child = tree.insert(Some(parent_a));

        assert_eq!(tree.children(parent_a), &[child]);

        tree.reparent(child, parent_b);

        assert!(tree.children(parent_a).is_empty());
        assert_eq!(tree.children(parent_b), &[child]);
        assert_eq!(tree.parent(child), Some(parent_b));
    }

    #[test]
    fn node_count() {
        let mut tree = NodeTree::new();
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());

        let root = tree.insert(None);
        assert_eq!(tree.len(), 1);

        let child = tree.insert(Some(root));
        assert_eq!(tree.len(), 2);

        tree.remove(child);
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn set_rect_marks_paint_dirty() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.clear_dirty(root);

        tree.set_rect(root, Rect::new(10.0, 20.0, 100.0, 200.0));

        let node = tree.get(root).unwrap();
        assert_eq!(node.rect, Rect::new(10.0, 20.0, 100.0, 200.0));
        assert!(node.paint_dirty);
    }

    #[test]
    fn set_visible_marks_paint_dirty() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.clear_dirty(root);

        tree.set_visible(root, false);

        let node = tree.get(root).unwrap();
        assert!(!node.visible);
        assert!(node.paint_dirty);
    }

    #[test]
    fn mark_layout_dirty_propagates_up() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));
        let grandchild = tree.insert(Some(child));

        tree.clear_dirty(root);
        tree.clear_dirty(child);
        tree.clear_dirty(grandchild);

        tree.mark_layout_dirty(grandchild);

        assert!(tree.get(grandchild).unwrap().layout_dirty);
        assert!(tree.get(child).unwrap().layout_dirty);
        assert!(tree.get(root).unwrap().layout_dirty);
    }

    #[test]
    fn mark_paint_dirty() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.clear_dirty(root);

        tree.mark_paint_dirty(root);

        assert!(tree.get(root).unwrap().paint_dirty);
    }

    #[test]
    fn rect_and_visible_getters() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);

        assert_eq!(tree.rect(root), Some(Rect::zero()));
        assert_eq!(tree.is_visible(root), Some(true));

        tree.set_rect(root, Rect::new(1.0, 2.0, 3.0, 4.0));
        tree.set_visible(root, false);

        assert_eq!(tree.rect(root), Some(Rect::new(1.0, 2.0, 3.0, 4.0)));
        assert_eq!(tree.is_visible(root), Some(false));
    }

    #[test]
    fn set_hit_test_transparent() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);

        assert_eq!(tree.is_hit_test_transparent(root), Some(false));

        tree.set_hit_test_transparent(root, true);
        assert_eq!(tree.is_hit_test_transparent(root), Some(true));
    }

    #[test]
    fn set_and_dispatch_event_handler() {
        use crate::event::{KeyEvent, KeyState};
        use crate::event_handler::{EventContext, EventHandler};
        use crate::shortcut::{Key, Modifiers};
        use std::cell::Cell;
        use std::rc::Rc;

        struct CountHandler {
            count: Rc<Cell<u32>>,
        }
        impl EventHandler for CountHandler {
            fn handle_key(&mut self, _: &KeyEvent, _: &mut EventContext) -> bool {
                self.count.set(self.count.get() + 1);
                true
            }
        }

        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        let count = Rc::new(Cell::new(0u32));
        tree.set_event_handler(
            root,
            CountHandler {
                count: count.clone(),
            },
        );

        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: KeyState::Pressed,
            text: Some("a".into()),
        };
        let consumed = tree.dispatch_key_event(root, &event);
        assert!(consumed);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn dispatch_to_node_without_handler_returns_false() {
        use crate::event::{KeyEvent, KeyState};
        use crate::shortcut::{Key, Modifiers};

        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));

        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: KeyState::Pressed,
            text: Some("a".into()),
        };
        let consumed = tree.dispatch_key_event(root, &event);
        assert!(!consumed);
    }

    #[test]
    fn key_dispatch_returns_redraw_and_clipboard_effects() {
        use crate::event::{KeyEvent, KeyState};
        use crate::event_handler::{EventContext, EventHandler};
        use crate::shortcut::{Key, Modifiers};

        struct ClipboardHandler;
        impl EventHandler for ClipboardHandler {
            fn handle_key(&mut self, _event: &KeyEvent, ctx: &mut EventContext) -> bool {
                assert_eq!(ctx.clipboard_get(), Some("paste me"));
                ctx.clipboard_set("copy me");
                ctx.request_redraw();
                true
            }
        }

        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_event_handler(root, ClipboardHandler);

        let event = KeyEvent {
            key: Key::V,
            modifiers: Modifiers::CTRL,
            state: KeyState::Pressed,
            text: None,
        };
        let result = tree.dispatch_key_event_with_context(root, &event, Some("paste me".into()));
        assert!(result.consumed);
        assert!(result.redraw_requested);
        assert_eq!(result.clipboard_write.as_deref(), Some("copy me"));
    }

    #[test]
    fn mouse_dispatch_returns_redraw_signal() {
        use crate::event::{ButtonState, KeyEvent, MouseButton, MouseEvent};
        use crate::event_handler::{EventContext, EventHandler};
        use crate::geometry::Point;
        use crate::shortcut::Modifiers;

        struct RedrawMouseHandler;
        impl EventHandler for RedrawMouseHandler {
            fn handle_key(&mut self, _event: &KeyEvent, _ctx: &mut EventContext) -> bool {
                false
            }

            fn handle_mouse(&mut self, _event: &MouseEvent, ctx: &mut EventContext) -> bool {
                ctx.request_redraw();
                true
            }
        }

        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.set_event_handler(root, RedrawMouseHandler);

        let event = MouseEvent {
            position: Point::new(10.0, 10.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 1,
            modifiers: Modifiers::empty(),
        };
        let result = tree.dispatch_mouse_event_with_context(root, &event);
        assert!(result.consumed);
        assert!(result.redraw_requested);
    }
}
