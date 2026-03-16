use crate::drag::DragState;
use crate::event::{ButtonState, MouseButton, MouseEvent};
use crate::focus::FocusState;
use crate::geometry::Point;
use crate::node::NodeId;
use crate::overlay::{OverlayId, OverlayStack};
use crate::paint::CommandList;
use crate::shortcut::Modifiers;
use crate::tree::EventDispatchResult;
use crate::tree::NodeTree;

pub struct Scene {
    tree: NodeTree,
    overlay_stack: OverlayStack,
    focus: FocusState,
    command_list: CommandList,
    captured_pointer: Option<NodeId>,
    drag_state: DragState,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            overlay_stack: OverlayStack::new(),
            focus: FocusState::new(),
            command_list: CommandList::new(),
            captured_pointer: None,
            drag_state: DragState::new(),
        }
    }

    pub fn tree(&self) -> &NodeTree {
        &self.tree
    }

    pub fn tree_mut(&mut self) -> &mut NodeTree {
        &mut self.tree
    }

    pub fn overlay_stack(&self) -> &OverlayStack {
        &self.overlay_stack
    }

    pub fn overlay_stack_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlay_stack
    }

    pub fn focus(&self) -> &FocusState {
        &self.focus
    }

    pub fn focus_mut(&mut self) -> &mut FocusState {
        &mut self.focus
    }

    pub fn request_focus(&mut self, id: NodeId) -> bool {
        if !self.tree.contains(id) {
            return false;
        }

        let previous = self.focus.focused();
        self.focus.request_focus(id);
        let current = self.focus.focused();
        if previous == current {
            return false;
        }

        self.tree.dispatch_focus_change(previous, current);
        true
    }

    pub fn clear_focus(&mut self) -> bool {
        let previous = self.focus.focused();
        if previous.is_none() {
            return false;
        }

        self.focus.release_focus();
        let current = self.focus.focused();
        self.tree.dispatch_focus_change(previous, current);
        true
    }

    pub fn blur_accessibility(&mut self, target: NodeId) -> bool {
        if self.focus.focused() != Some(target) {
            return false;
        }

        self.clear_focus()
    }

    pub fn activate_accessibility(&mut self, target: NodeId) -> EventDispatchResult {
        if !self.tree.contains(target) {
            return EventDispatchResult::default();
        }

        let focus_changed = self.request_focus(target);
        let rect = self.tree.rect(target).unwrap_or(crate::Rect::zero());
        let event = MouseEvent {
            position: Point::new(rect.width * 0.5, rect.height * 0.5),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 1,
            modifiers: Modifiers::empty(),
        };

        let mut result = self.tree.dispatch_mouse_event_with_context(target, &event);
        result.redraw_requested |= focus_changed;
        result
    }

    pub fn handle_accessibility_action(
        &mut self,
        target: NodeId,
        action: &crate::AccessibilityAction,
    ) -> EventDispatchResult {
        if !self.tree.contains(target) {
            return EventDispatchResult::default();
        }

        let focus_changed = self.request_focus(target);
        let mut result = self
            .tree
            .dispatch_accessibility_action_with_context(target, action);
        result.redraw_requested |= focus_changed;
        result
    }

    pub fn drag_state(&self) -> &DragState {
        &self.drag_state
    }

    pub fn drag_state_mut(&mut self) -> &mut DragState {
        &mut self.drag_state
    }

    pub fn push_overlay(&mut self) -> OverlayId {
        self.overlay_stack.push_overlay()
    }

    pub fn layout(&mut self) {
        self.tree.run_layout();
        self.overlay_stack
            .for_each_tree_mut(|tree| tree.run_layout());
    }

    pub fn paint(&mut self) {
        self.command_list.clear();
        self.tree.run_paint(&mut self.command_list);
        self.overlay_stack
            .for_each_tree_mut(|tree| tree.run_paint(&mut self.command_list));
    }

    pub fn paint_uncached(&mut self) {
        self.command_list.clear();
        self.tree.run_paint_uncached(&mut self.command_list);
        self.overlay_stack
            .for_each_tree_mut(|tree| tree.run_paint_uncached(&mut self.command_list));
    }

    pub fn invalidate_all(&mut self) {
        self.tree.invalidate_all_paint();
    }

    pub fn capture_pointer(&mut self, node_id: NodeId) {
        self.captured_pointer = Some(node_id);
    }

    pub fn release_pointer(&mut self) {
        self.captured_pointer = None;
    }

    pub fn pointer_captured_by(&self) -> Option<NodeId> {
        self.captured_pointer
    }

    pub fn hit_test(&self, point: Point) -> Option<NodeId> {
        if let Some(captured) = self.captured_pointer {
            if self.tree.contains(captured) {
                return Some(captured);
            }
        }
        if let Some((_overlay_id, node_id)) = self.overlay_stack.hit_test(point) {
            return Some(node_id);
        }
        self.tree.hit_test(point)
    }

    pub fn commands(&self) -> &CommandList {
        &self.command_list
    }

    pub fn commands_mut(&mut self) -> &mut CommandList {
        &mut self.command_list
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_handler::{EventContext, EventHandler};
    use crate::geometry::Rect;
    use crate::layout::{Direction, StackLayout};
    use crate::paint::Color;
    use crate::painter::Painter;
    use crate::{AccessibilityAction, AccessibilityNode, AccessibilityRole, KeyEvent, MouseEvent};
    use std::cell::Cell;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct TestPainter {
        color: Color,
    }

    impl Painter for TestPainter {
        fn paint(&self, rect: Rect, commands: &mut CommandList) {
            commands.fill_rect(rect, self.color);
        }
    }

    #[test]
    fn scene_layout_then_paint() {
        let mut scene = Scene::new();

        let root = scene.tree_mut().insert(None);
        let child = scene.tree_mut().insert(Some(root));

        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        scene.tree_mut().set_layout(
            root,
            StackLayout {
                direction: Direction::Vertical,
                spacing: 0.0,
            },
        );
        scene.tree_mut().set_painter(
            root,
            TestPainter {
                color: Color::rgb(255, 0, 0),
            },
        );
        scene.tree_mut().set_painter(
            child,
            TestPainter {
                color: Color::rgb(0, 255, 0),
            },
        );

        scene.layout();
        scene.paint();

        assert!(!scene.commands().commands().is_empty());
    }

    #[test]
    fn scene_hit_test_checks_overlays_first() {
        let mut scene = Scene::new();

        let main_root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(main_root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let overlay_id = scene.push_overlay();
        let tree = scene
            .overlay_stack_mut()
            .overlay_tree_mut(overlay_id)
            .unwrap();
        let overlay_root = tree.insert(None);
        tree.set_rect(overlay_root, Rect::new(0.0, 0.0, 100.0, 100.0));

        let hit = scene.hit_test(Point::new(50.0, 50.0));
        assert_eq!(hit, Some(overlay_root));
    }

    #[test]
    fn scene_hit_test_falls_through_to_main_tree() {
        let mut scene = Scene::new();

        let main_root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(main_root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let overlay_id = scene.push_overlay();
        let tree = scene
            .overlay_stack_mut()
            .overlay_tree_mut(overlay_id)
            .unwrap();
        let overlay_root = tree.insert(None);
        tree.set_rect(overlay_root, Rect::new(0.0, 0.0, 50.0, 50.0));

        let hit = scene.hit_test(Point::new(200.0, 200.0));
        assert_eq!(hit, Some(main_root));

        let _ = overlay_root;
    }

    #[test]
    fn scene_focus() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);

        scene.request_focus(root);
        assert_eq!(scene.focus().focused(), Some(root));
    }

    #[test]
    fn scene_request_focus_notifies_handlers_and_clear_blurs() {
        struct FocusHandler {
            focus_state: Rc<Cell<bool>>,
            focus_events: Rc<Cell<u32>>,
        }

        impl EventHandler for FocusHandler {
            fn handle_key(&mut self, _event: &KeyEvent, _ctx: &mut EventContext) -> bool {
                false
            }

            fn handle_focus(&mut self, gained: bool) {
                self.focus_state.set(gained);
                self.focus_events.set(self.focus_events.get() + 1);
            }
        }

        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        let child = scene.tree_mut().insert(Some(root));

        let focus_state = Rc::new(Cell::new(false));
        let focus_events = Rc::new(Cell::new(0u32));
        scene.tree_mut().set_event_handler(
            child,
            FocusHandler {
                focus_state: focus_state.clone(),
                focus_events: focus_events.clone(),
            },
        );

        assert!(scene.request_focus(child));
        assert_eq!(scene.focus().focused(), Some(child));
        assert!(focus_state.get());
        assert_eq!(focus_events.get(), 1);

        assert!(!scene.request_focus(child));
        assert_eq!(focus_events.get(), 1);

        assert!(scene.blur_accessibility(child));
        assert_eq!(scene.focus().focused(), None);
        assert!(!focus_state.get());
        assert_eq!(focus_events.get(), 2);
    }

    #[test]
    fn accessibility_activation_focuses_and_clicks_center() {
        struct AccessibilityHandler {
            focused: Rc<Cell<bool>>,
            clicks: Rc<Cell<u32>>,
            last_position: Rc<Cell<Point>>,
        }

        impl EventHandler for AccessibilityHandler {
            fn handle_key(&mut self, _event: &KeyEvent, _ctx: &mut EventContext) -> bool {
                false
            }

            fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
                self.clicks.set(self.clicks.get() + 1);
                self.last_position.set(event.position);
                ctx.request_redraw();
                true
            }

            fn handle_focus(&mut self, gained: bool) {
                self.focused.set(gained);
            }
        }

        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        let child = scene.tree_mut().insert(Some(root));
        scene
            .tree_mut()
            .set_rect(child, Rect::new(10.0, 20.0, 80.0, 30.0));

        let focused = Rc::new(Cell::new(false));
        let clicks = Rc::new(Cell::new(0u32));
        let last_position = Rc::new(Cell::new(Point::new(0.0, 0.0)));
        scene.tree_mut().set_event_handler(
            child,
            AccessibilityHandler {
                focused: focused.clone(),
                clicks: clicks.clone(),
                last_position: last_position.clone(),
            },
        );

        let result = scene.activate_accessibility(child);
        assert!(result.consumed);
        assert!(result.redraw_requested);
        assert_eq!(scene.focus().focused(), Some(child));
        assert!(focused.get());
        assert_eq!(clicks.get(), 1);
        assert_eq!(last_position.get(), Point::new(40.0, 15.0));
    }

    #[test]
    fn scene_accessibility_text_action_focuses_and_updates_value() {
        struct TextAccessibilityHandler {
            value: Rc<RefCell<String>>,
        }

        impl EventHandler for TextAccessibilityHandler {
            fn handle_key(&mut self, _event: &KeyEvent, _ctx: &mut EventContext) -> bool {
                false
            }

            fn handle_accessibility_action(
                &mut self,
                action: &AccessibilityAction,
                ctx: &mut EventContext,
            ) -> bool {
                let next_value = match action {
                    AccessibilityAction::SetValue(value) => value.clone(),
                    AccessibilityAction::ReplaceSelectedText(value) => value.clone(),
                    AccessibilityAction::SetTextSelection(_) => String::new(),
                };
                if !next_value.is_empty() {
                    *self.value.borrow_mut() = next_value.clone();
                    ctx.set_accessibility_value(Some(next_value));
                }
                ctx.request_redraw();
                true
            }
        }

        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        let child = scene.tree_mut().insert(Some(root));
        scene.tree_mut().set_accessibility(
            child,
            AccessibilityNode::new(AccessibilityRole::TextInput).supports_text_input_actions(),
        );

        let value = Rc::new(RefCell::new(String::new()));
        scene.tree_mut().set_event_handler(
            child,
            TextAccessibilityHandler {
                value: value.clone(),
            },
        );

        let result = scene
            .handle_accessibility_action(child, &AccessibilityAction::SetValue("Hello".into()));
        assert!(result.consumed);
        assert!(result.redraw_requested);
        assert_eq!(scene.focus().focused(), Some(child));
        assert_eq!(value.borrow().as_str(), "Hello");
        assert_eq!(
            scene
                .tree()
                .accessibility(child)
                .and_then(|node| node.value.as_deref()),
            Some("Hello")
        );
    }

    #[test]
    fn pointer_capture_overrides_hit_test() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let child = scene.tree_mut().insert(Some(root));
        scene
            .tree_mut()
            .set_rect(child, Rect::new(0.0, 0.0, 100.0, 100.0));

        scene.capture_pointer(child);
        let hit = scene.hit_test(Point::new(400.0, 400.0));
        assert_eq!(hit, Some(child));
        assert_eq!(scene.pointer_captured_by(), Some(child));
    }

    #[test]
    fn pointer_capture_release() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 500.0, 500.0));

        scene.capture_pointer(root);
        scene.release_pointer();
        assert_eq!(scene.pointer_captured_by(), None);

        let hit = scene.hit_test(Point::new(250.0, 250.0));
        assert_eq!(hit, Some(root));
    }

    #[test]
    fn pointer_capture_invalid_node_falls_through() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let child = scene.tree_mut().insert(Some(root));
        scene
            .tree_mut()
            .set_rect(child, Rect::new(0.0, 0.0, 100.0, 100.0));

        scene.capture_pointer(child);
        scene.tree_mut().remove(child);

        let hit = scene.hit_test(Point::new(50.0, 50.0));
        assert_eq!(hit, Some(root));
    }

    #[test]
    fn scene_paint_clears_and_rebuilds_commands() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));
        scene.tree_mut().set_painter(
            root,
            TestPainter {
                color: Color::rgb(255, 0, 0),
            },
        );

        scene.paint();
        let first_count = scene.commands().commands().len();

        scene.paint();
        let second_count = scene.commands().commands().len();

        assert_eq!(first_count, second_count);
        assert!(first_count > 0);
    }
}
