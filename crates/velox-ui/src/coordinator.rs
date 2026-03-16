use std::collections::HashMap;

use velox_scene::{ButtonState, KeyEvent, MouseButton, MouseEvent, NodeId, Point, ScrollEvent};

use crate::action::{ActionRegistry, Keymap, Keystroke};
use crate::active::ActiveManager;
use crate::cursor::CursorManager;
use crate::dispatch::{DispatchNodeData, DispatchTree};
use crate::drag::{DragManager, DragPhaseUi};
use crate::focus::{FocusChange, FocusManager};
use crate::hover::{HoverChange, HoverManager};
use crate::interactive::ClickEvent;
use crate::reconciler::ReconcilerSlot;
use crate::scroll::{ScrollAxis, ScrollState};
use crate::style::{CursorStyle, Overflow};

const CLICK_DISTANCE_THRESHOLD: f32 = 4.0;
const DOUBLE_CLICK_MS: u128 = 500;

pub struct EventResult {
    pub needs_redraw: bool,
    pub cursor_changed: Option<CursorStyle>,
}

impl EventResult {
    fn none() -> Self {
        Self {
            needs_redraw: false,
            cursor_changed: None,
        }
    }

    fn redraw() -> Self {
        Self {
            needs_redraw: true,
            cursor_changed: None,
        }
    }
}

pub struct UiCoordinator {
    dispatch: DispatchTree,
    hover: HoverManager,
    active: ActiveManager,
    focus: FocusManager,
    cursor: CursorManager,
    drag: DragManager,
    keymap: Keymap,
    actions: ActionRegistry,
    scroll_states: HashMap<NodeId, ScrollState>,
    last_mousedown_pos: Option<Point>,
    last_mousedown_node: Option<NodeId>,
    last_click_time: Option<std::time::Instant>,
    last_click_node: Option<NodeId>,
    click_count: u32,
}

impl UiCoordinator {
    pub fn new() -> Self {
        Self {
            dispatch: DispatchTree::new(),
            hover: HoverManager::new(),
            active: ActiveManager::new(),
            focus: FocusManager::new(),
            cursor: CursorManager::new(),
            drag: DragManager::new(),
            keymap: Keymap::new(),
            actions: ActionRegistry::new(),
            scroll_states: HashMap::new(),
            last_mousedown_pos: None,
            last_mousedown_node: None,
            last_click_time: None,
            last_click_node: None,
            click_count: 0,
        }
    }

    pub fn dispatch_tree(&self) -> &DispatchTree {
        &self.dispatch
    }

    pub fn dispatch_tree_mut(&mut self) -> &mut DispatchTree {
        &mut self.dispatch
    }

    pub fn focus_manager(&self) -> &FocusManager {
        &self.focus
    }

    pub fn focus_manager_mut(&mut self) -> &mut FocusManager {
        &mut self.focus
    }

    pub fn hover_manager(&self) -> &HoverManager {
        &self.hover
    }

    pub fn hover_manager_mut(&mut self) -> &mut HoverManager {
        &mut self.hover
    }

    pub fn active_manager(&self) -> &ActiveManager {
        &self.active
    }

    pub fn drag_manager(&self) -> &DragManager {
        &self.drag
    }

    pub fn drag_manager_mut(&mut self) -> &mut DragManager {
        &mut self.drag
    }

    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }

    pub fn actions(&self) -> &ActionRegistry {
        &self.actions
    }

    pub fn actions_mut(&mut self) -> &mut ActionRegistry {
        &mut self.actions
    }

    pub fn scroll_state(&self, node: NodeId) -> Option<&ScrollState> {
        self.scroll_states.get(&node)
    }

    pub fn scroll_state_mut(&mut self, node: NodeId) -> Option<&mut ScrollState> {
        self.scroll_states.get_mut(&node)
    }

    pub fn hovered_node(&self) -> Option<NodeId> {
        self.hover.hovered()
    }

    pub fn active_node(&self) -> Option<NodeId> {
        self.active.active()
    }

    pub fn focused_node(&self) -> Option<NodeId> {
        self.focus.focused()
    }

    pub fn rebuild_dispatch(&mut self) {
        self.dispatch.clear();
        self.actions.clear();
    }

    pub fn register_node(&mut self, node: NodeId, parent: Option<NodeId>, data: DispatchNodeData) {
        if data.scrollable_x || data.scrollable_y {
            self.scroll_states.entry(node).or_insert_with(|| {
                let axis = match (data.scrollable_x, data.scrollable_y) {
                    (true, true) => ScrollAxis::Both,
                    (true, false) => ScrollAxis::Horizontal,
                    _ => ScrollAxis::Vertical,
                };
                ScrollState::new(axis)
            });
        }

        if data.focusable {
            let tab_index = data.tab_index.unwrap_or(0);
            self.focus.tab_stops_mut().insert(node, tab_index);
        }

        self.dispatch.register(node, parent, data);
    }

    pub fn build_dispatch_from_slots(&mut self, slots: &mut [ReconcilerSlot]) {
        self.rebuild_dispatch();
        self.focus.tab_stops_mut().clear();
        for slot in slots.iter_mut() {
            self.register_slot(slot, None);
        }
        self.prune_scroll_states();
        self.prune_interaction_state();
    }

    fn register_slot(&mut self, slot: &mut ReconcilerSlot, parent: Option<NodeId>) {
        let handlers = slot.handlers.take().unwrap_or_default();
        let scrollable_x = slot
            .style
            .overflow_x == Some(Overflow::Scroll);
        let scrollable_y = slot
            .style
            .overflow_y == Some(Overflow::Scroll);

        let data = DispatchNodeData {
            cursor: slot.style.cursor,
            key_context: handlers.key_context.clone(),
            focusable: handlers.focusable,
            tab_index: handlers.tab_index,
            scrollable_x,
            scrollable_y,
            handlers,
        };

        self.register_node(slot.node_id, parent, data);

        for child in slot.children.iter_mut() {
            self.register_slot(child, Some(slot.node_id));
        }
    }

    pub fn handle_mouse_move(&mut self, target: Option<NodeId>, position: Point) -> EventResult {
        let mut needs_redraw = false;

        if self.drag.phase() == DragPhaseUi::Pending || self.drag.phase() == DragPhaseUi::Active {
            let activated = self.drag.mouse_move(position, || None);
            if activated || self.drag.phase() == DragPhaseUi::Active {
                return EventResult {
                    needs_redraw: true,
                    cursor_changed: Some(CursorStyle::Grabbing),
                };
            }
        }

        let hover_change = self.hover.set_hovered(target);
        if hover_change.entered.is_some() || hover_change.exited.is_some() {
            needs_redraw = true;
            self.fire_hover_handlers(&hover_change);
        }

        let cursor_changed = if let Some(node) = target {
            self.cursor.resolve(node, &self.dispatch)
        } else {
            self.cursor.reset()
        };

        EventResult {
            needs_redraw,
            cursor_changed,
        }
    }

    pub fn handle_mouse_down(
        &mut self,
        target: NodeId,
        position: Point,
        button: MouseButton,
    ) -> EventResult {
        self.active.set_active(Some(target));
        self.last_mousedown_pos = Some(position);
        self.last_mousedown_node = Some(target);

        if let Some(data) = self.dispatch.get(target)
            && data.focusable {
                let change = self.focus.request_focus(target);
                self.fire_focus_handlers(&change);
            }

        let has_drag_handler = self
            .dispatch
            .get(target)
            .is_some_and(|d| d.handlers.on_drag.is_some());
        if has_drag_handler {
            self.drag.begin_pending(target, position);
        }

        self.fire_mouse_down(target, position, button);
        EventResult::redraw()
    }

    pub fn handle_mouse_up(
        &mut self,
        target: Option<NodeId>,
        position: Point,
        button: MouseButton,
    ) -> EventResult {
        self.active.clear();

        if self.drag.phase() == DragPhaseUi::Active {
            let _finish = self.drag.finish();
            return EventResult {
                needs_redraw: true,
                cursor_changed: Some(CursorStyle::Default),
            };
        }
        self.drag.cancel();

        if let Some(down_node) = self.last_mousedown_node
            && let Some(up_node) = target {
                let is_same_target = down_node == up_node || self.is_ancestor(down_node, up_node);
                if is_same_target
                    && let Some(down_pos) = self.last_mousedown_pos {
                        let dx = position.x - down_pos.x;
                        let dy = position.y - down_pos.y;
                        if (dx * dx + dy * dy).sqrt() < CLICK_DISTANCE_THRESHOLD {
                            self.synthesize_click(down_node, position, button);
                        }
                    }
            }

        self.last_mousedown_pos = None;
        self.last_mousedown_node = None;

        if let Some(node) = target {
            self.fire_mouse_up(node, position, button);
        }

        EventResult::redraw()
    }

    pub fn handle_scroll(&mut self, target: NodeId, event: &ScrollEvent) -> EventResult {
        let mut current = Some(target);
        while let Some(node) = current {
            if let Some(scroll_state) = self.scroll_states.get_mut(&node) {
                scroll_state.scroll_by(event.delta_x, -event.delta_y);

                if let Some(data) = self.dispatch.get(node)
                    && let Some(ref handler) = data.handlers.on_scroll {
                        handler(event);
                    }
                return EventResult::redraw();
            }
            current = self.dispatch.parent(node);
        }

        if let Some(data) = self.dispatch.get(target)
            && let Some(ref handler) = data.handlers.on_scroll {
                handler(event);
                return EventResult::redraw();
            }

        EventResult::none()
    }

    pub fn handle_key_down(&mut self, event: &KeyEvent) -> EventResult {
        let focused = match self.focus.focused() {
            Some(f) => f,
            None => return EventResult::none(),
        };

        let keystroke = Keystroke {
            key: event.key,
            modifiers: event.modifiers,
        };

        let contexts: Vec<String> = self.collect_key_contexts(focused);
        if let Some(action) = self.keymap.match_keystroke(&keystroke, &contexts) {
            let action_clone = action.boxed_clone();
            let bubble_path = self.dispatch.bubble_path(focused);
            for node in &bubble_path {
                if self.actions.dispatch(*node, action_clone.as_ref()) {
                    return EventResult::redraw();
                }
            }
        }

        let bubble_path = self.dispatch.bubble_path(focused);
        for node in bubble_path {
            if let Some(data) = self.dispatch.get(node)
                && let Some(ref handler) = data.handlers.on_key_down {
                    handler(event);
                    return EventResult::redraw();
                }
        }

        EventResult::none()
    }

    pub fn handle_tab(&mut self, shift: bool) -> EventResult {
        let next = if shift {
            self.focus.prev_focus()
        } else {
            self.focus.next_focus()
        };

        if let Some(node) = next {
            let change = self.focus.request_focus(node);
            self.fire_focus_handlers(&change);
            return EventResult::redraw();
        }

        EventResult::none()
    }

    pub fn handle_accessibility_focus(&mut self, target: NodeId) -> EventResult {
        if self.dispatch.get(target).is_none() {
            return EventResult::none();
        }

        let change = self.focus.request_focus(target);
        if change.gained.is_none() && change.lost.is_none() {
            return EventResult::none();
        }
        self.fire_focus_handlers(&change);
        EventResult::redraw()
    }

    pub fn handle_accessibility_blur(&mut self, target: NodeId) -> EventResult {
        if self.focus.focused() != Some(target) {
            return EventResult::none();
        }

        let change = self.focus.clear_focus();
        if change.gained.is_none() && change.lost.is_none() {
            return EventResult::none();
        }
        self.fire_focus_handlers(&change);
        EventResult::redraw()
    }

    pub fn handle_accessibility_click(&mut self, target: NodeId, position: Point) -> EventResult {
        let Some(data) = self.dispatch.get(target) else {
            return EventResult::none();
        };

        if data.focusable {
            let change = self.focus.request_focus(target);
            self.fire_focus_handlers(&change);
        }
        self.synthesize_click(target, position, MouseButton::Left);
        EventResult::redraw()
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let mut needs_redraw = false;
        let node_ids: Vec<NodeId> = self.scroll_states.keys().copied().collect();
        for node_id in node_ids {
            if let Some(scroll) = self.scroll_states.get_mut(&node_id)
                && scroll.tick(dt) {
                    needs_redraw = true;
                }
        }
        needs_redraw
    }

    pub fn has_animations(&self) -> bool {
        self.scroll_states.values().any(|s| s.is_animating())
    }

    pub fn prune_scroll_states(&mut self) {
        self.scroll_states
            .retain(|node, _| self.dispatch.get(*node).is_some());
    }

    fn prune_interaction_state(&mut self) {
        if self
            .focus
            .focused()
            .is_some_and(|node| self.dispatch.get(node).is_none())
        {
            let _ = self.focus.clear_focus();
        }
        if self
            .hover
            .hovered()
            .is_some_and(|node| self.dispatch.get(node).is_none())
        {
            let _ = self.hover.set_hovered(None);
        }
        if self
            .active
            .active()
            .is_some_and(|node| self.dispatch.get(node).is_none())
        {
            self.active.clear();
        }
    }

    fn fire_hover_handlers(&self, change: &HoverChange) {
        if let Some(exited) = change.exited
            && let Some(data) = self.dispatch.get(exited)
                && let Some(ref handler) = data.handlers.on_hover {
                    handler(false);
                }
        if let Some(entered) = change.entered
            && let Some(data) = self.dispatch.get(entered)
                && let Some(ref handler) = data.handlers.on_hover {
                    handler(true);
                }
    }

    fn fire_focus_handlers(&self, change: &FocusChange) {
        if let Some(lost) = change.lost
            && let Some(data) = self.dispatch.get(lost)
                && let Some(ref handler) = data.handlers.on_focus {
                    handler(false);
                }
        if let Some(gained) = change.gained
            && let Some(data) = self.dispatch.get(gained)
                && let Some(ref handler) = data.handlers.on_focus {
                    handler(true);
                }
    }

    fn fire_mouse_down(&self, target: NodeId, position: Point, button: MouseButton) {
        let bubble_path = self.dispatch.bubble_path(target);
        for node in bubble_path {
            if let Some(data) = self.dispatch.get(node)
                && let Some(ref handler) = data.handlers.on_mouse_down {
                    let event = MouseEvent {
                        position,
                        button,
                        state: ButtonState::Pressed,
                        click_count: 1,
                        modifiers: velox_scene::Modifiers::empty(),
                    };
                    handler(&event);
                    return;
                }
        }
    }

    fn fire_mouse_up(&self, target: NodeId, position: Point, button: MouseButton) {
        let bubble_path = self.dispatch.bubble_path(target);
        for node in bubble_path {
            if let Some(data) = self.dispatch.get(node)
                && let Some(ref handler) = data.handlers.on_mouse_up {
                    let event = MouseEvent {
                        position,
                        button,
                        state: ButtonState::Released,
                        click_count: 1,
                        modifiers: velox_scene::Modifiers::empty(),
                    };
                    handler(&event);
                    return;
                }
        }
    }

    fn synthesize_click(&mut self, target: NodeId, position: Point, button: MouseButton) {
        let now = std::time::Instant::now();
        if let Some(last_time) = self.last_click_time {
            if self.last_click_node == Some(target)
                && now.duration_since(last_time).as_millis() < DOUBLE_CLICK_MS
            {
                self.click_count += 1;
            } else {
                self.click_count = 1;
            }
        } else {
            self.click_count = 1;
        }
        self.last_click_time = Some(now);
        self.last_click_node = Some(target);

        let click_event = ClickEvent {
            position,
            button,
            click_count: self.click_count,
        };

        let bubble_path = self.dispatch.bubble_path(target);
        for node in bubble_path {
            if let Some(data) = self.dispatch.get(node)
                && let Some(ref handler) = data.handlers.on_click {
                    handler(&click_event);
                    return;
                }
        }
    }

    fn collect_key_contexts(&self, node: NodeId) -> Vec<String> {
        let mut contexts = Vec::new();
        let path = self.dispatch.bubble_path(node);
        for n in path {
            if let Some(data) = self.dispatch.get(n)
                && let Some(ref ctx) = data.key_context {
                    contexts.push(ctx.clone());
                }
        }
        contexts
    }

    fn is_ancestor(&self, ancestor: NodeId, descendant: NodeId) -> bool {
        let mut current = self.dispatch.parent(descendant);
        while let Some(node) = current {
            if node == ancestor {
                return true;
            }
            current = self.dispatch.parent(node);
        }
        false
    }
}

impl Default for UiCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactive::EventHandlers;
    use std::cell::Cell;
    use std::rc::Rc;

    fn node_id(n: u64) -> NodeId {
        NodeId::from(slotmap::KeyData::from_ffi(n))
    }

    #[test]
    fn coordinator_tracks_hover() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(node, None, DispatchNodeData::default());

        let result = coord.handle_mouse_move(Some(node), Point::new(10.0, 10.0));
        assert!(result.needs_redraw);
        assert_eq!(coord.hovered_node(), Some(node));
    }

    #[test]
    fn coordinator_hover_exit() {
        let mut coord = UiCoordinator::new();
        let a = node_id(1);
        let b = node_id(2);
        coord.register_node(a, None, DispatchNodeData::default());
        coord.register_node(b, None, DispatchNodeData::default());

        coord.handle_mouse_move(Some(a), Point::new(10.0, 10.0));
        let result = coord.handle_mouse_move(Some(b), Point::new(20.0, 20.0));
        assert!(result.needs_redraw);
        assert_eq!(coord.hovered_node(), Some(b));
    }

    #[test]
    fn coordinator_mousedown_sets_active_and_focus() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(
            node,
            None,
            DispatchNodeData {
                focusable: true,
                ..Default::default()
            },
        );

        coord.handle_mouse_down(node, Point::new(10.0, 10.0), MouseButton::Left);
        assert_eq!(coord.active_node(), Some(node));
        assert_eq!(coord.focused_node(), Some(node));
    }

    #[test]
    fn coordinator_mouseup_clears_active() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(node, None, DispatchNodeData::default());

        coord.handle_mouse_down(node, Point::new(10.0, 10.0), MouseButton::Left);
        coord.handle_mouse_up(Some(node), Point::new(10.0, 10.0), MouseButton::Left);
        assert_eq!(coord.active_node(), None);
    }

    #[test]
    fn coordinator_click_synthesis() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        let clicked = Rc::new(Cell::new(false));
        let flag = clicked.clone();

        let mut handlers = EventHandlers::default();
        handlers.on_click = Some(Rc::new(move |_| flag.set(true)));

        coord.register_node(
            node,
            None,
            DispatchNodeData {
                handlers,
                ..Default::default()
            },
        );

        coord.handle_mouse_down(node, Point::new(10.0, 10.0), MouseButton::Left);
        coord.handle_mouse_up(Some(node), Point::new(10.0, 10.0), MouseButton::Left);
        assert!(clicked.get());
    }

    #[test]
    fn coordinator_scroll_creates_state() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(
            node,
            None,
            DispatchNodeData {
                scrollable_y: true,
                ..Default::default()
            },
        );

        assert!(coord.scroll_state(node).is_some());
    }

    #[test]
    fn coordinator_scroll_event_updates_offset() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(
            node,
            None,
            DispatchNodeData {
                scrollable_y: true,
                ..Default::default()
            },
        );

        if let Some(ss) = coord.scroll_state_mut(node) {
            ss.set_viewport_size(400.0, 600.0);
            ss.set_content_size(400.0, 2000.0);
        }

        let event = ScrollEvent {
            delta_x: 0.0,
            delta_y: -50.0,
            modifiers: velox_scene::Modifiers::empty(),
        };
        let result = coord.handle_scroll(node, &event);
        assert!(result.needs_redraw);

        let offset = coord.scroll_state(node).unwrap().offset_y();
        assert!(offset > 0.0);
    }

    #[test]
    fn coordinator_cursor_resolution() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(
            node,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Pointer),
                ..Default::default()
            },
        );

        let result = coord.handle_mouse_move(Some(node), Point::new(10.0, 10.0));
        assert_eq!(result.cursor_changed, Some(CursorStyle::Pointer));
    }

    #[test]
    fn coordinator_tab_cycling() {
        let mut coord = UiCoordinator::new();
        let a = node_id(1);
        let b = node_id(2);
        coord.register_node(
            a,
            None,
            DispatchNodeData {
                focusable: true,
                ..Default::default()
            },
        );
        coord.register_node(
            b,
            None,
            DispatchNodeData {
                focusable: true,
                ..Default::default()
            },
        );

        let result = coord.handle_tab(false);
        assert!(result.needs_redraw);
        assert_eq!(coord.focused_node(), Some(a));

        coord.handle_tab(false);
        assert_eq!(coord.focused_node(), Some(b));

        coord.handle_tab(false);
        assert_eq!(coord.focused_node(), Some(a));
    }

    #[test]
    fn coordinator_tick_advances_scroll() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        coord.register_node(
            node,
            None,
            DispatchNodeData {
                scrollable_y: true,
                ..Default::default()
            },
        );

        if let Some(ss) = coord.scroll_state_mut(node) {
            ss.set_viewport_size(400.0, 600.0);
            ss.set_content_size(400.0, 2000.0);
            ss.scroll_by(0.0, 100.0);
        }

        assert!(coord.has_animations());
        let needs_redraw = coord.tick(1.0 / 60.0);
        assert!(needs_redraw);
    }

    #[test]
    fn coordinator_hover_handler_called() {
        let mut coord = UiCoordinator::new();
        let node = node_id(1);
        let hover_state = Rc::new(Cell::new(false));
        let flag = hover_state.clone();

        let mut handlers = EventHandlers::default();
        handlers.on_hover = Some(Rc::new(move |entered| flag.set(entered)));

        coord.register_node(
            node,
            None,
            DispatchNodeData {
                handlers,
                ..Default::default()
            },
        );

        coord.handle_mouse_move(Some(node), Point::new(10.0, 10.0));
        assert!(hover_state.get());

        coord.handle_mouse_move(None, Point::new(-1.0, -1.0));
        assert!(!hover_state.get());
    }

    #[test]
    fn coordinator_bubble_click_to_parent() {
        let mut coord = UiCoordinator::new();
        let parent = node_id(1);
        let child = node_id(2);
        let clicked = Rc::new(Cell::new(false));
        let flag = clicked.clone();

        let mut parent_handlers = EventHandlers::default();
        parent_handlers.on_click = Some(Rc::new(move |_| flag.set(true)));

        coord.register_node(
            parent,
            None,
            DispatchNodeData {
                handlers: parent_handlers,
                ..Default::default()
            },
        );
        coord.register_node(child, Some(parent), DispatchNodeData::default());

        coord.handle_mouse_down(child, Point::new(10.0, 10.0), MouseButton::Left);
        coord.handle_mouse_up(Some(child), Point::new(10.0, 10.0), MouseButton::Left);
        assert!(clicked.get());
    }

    #[test]
    fn coordinator_scroll_bubbles_to_scrollable_ancestor() {
        let mut coord = UiCoordinator::new();
        let scrollable_parent = node_id(1);
        let child = node_id(2);

        coord.register_node(
            scrollable_parent,
            None,
            DispatchNodeData {
                scrollable_y: true,
                ..Default::default()
            },
        );
        coord.register_node(child, Some(scrollable_parent), DispatchNodeData::default());

        if let Some(ss) = coord.scroll_state_mut(scrollable_parent) {
            ss.set_viewport_size(400.0, 600.0);
            ss.set_content_size(400.0, 2000.0);
        }

        let event = ScrollEvent {
            delta_x: 0.0,
            delta_y: -50.0,
            modifiers: velox_scene::Modifiers::empty(),
        };
        let result = coord.handle_scroll(child, &event);
        assert!(result.needs_redraw);
        assert!(coord.scroll_state(scrollable_parent).unwrap().offset_y() > 0.0);
    }
}
