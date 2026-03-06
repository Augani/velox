use velox_reactive::{Event, Subscription};

use crate::node::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusChange {
    pub lost: Option<NodeId>,
    pub gained: Option<NodeId>,
}

struct FocusScope {
    allowed_nodes: Vec<NodeId>,
    previous_focus: Option<NodeId>,
}

pub struct FocusState {
    focused: Option<NodeId>,
    change_event: Event<FocusChange>,
    scopes: Vec<FocusScope>,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            focused: None,
            change_event: Event::new(),
            scopes: Vec::new(),
        }
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    pub fn request_focus(&mut self, id: NodeId) {
        if self.focused == Some(id) {
            return;
        }
        if let Some(scope) = self.scopes.last() {
            if !scope.allowed_nodes.contains(&id) {
                return;
            }
        }
        let lost = self.focused.take();
        self.focused = Some(id);
        self.change_event.emit(FocusChange {
            lost,
            gained: Some(id),
        });
    }

    pub fn release_focus(&mut self) {
        let Some(lost_id) = self.focused.take() else {
            return;
        };
        self.change_event.emit(FocusChange {
            lost: Some(lost_id),
            gained: None,
        });
    }

    pub fn push_scope(&mut self, allowed: Vec<NodeId>) {
        let previous_focus = self.focused;
        self.scopes.push(FocusScope {
            allowed_nodes: allowed,
            previous_focus,
        });
        self.release_focus();
    }

    pub fn pop_scope(&mut self) {
        let Some(scope) = self.scopes.pop() else {
            return;
        };
        if let Some(prev) = scope.previous_focus {
            self.focused = Some(prev);
            self.change_event.emit(FocusChange {
                lost: self.focused,
                gained: Some(prev),
            });
        }
    }

    pub fn on_focus_change(&self, callback: impl Fn(&FocusChange) + 'static) -> Subscription {
        self.change_event.subscribe(callback)
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use std::cell::Cell;
    use std::rc::Rc;

    fn make_slot_map() -> SlotMap<NodeId, ()> {
        SlotMap::with_key()
    }

    #[test]
    fn initially_no_focus() {
        let state = FocusState::new();
        assert_eq!(state.focused(), None);
    }

    #[test]
    fn request_and_release_focus() {
        let mut sm = make_slot_map();
        let mut state = FocusState::new();
        let id = sm.insert(());

        state.request_focus(id);
        assert_eq!(state.focused(), Some(id));

        state.release_focus();
        assert_eq!(state.focused(), None);
    }

    #[test]
    fn focus_change_emits_event() {
        let mut sm = make_slot_map();
        let mut state = FocusState::new();
        let id = sm.insert(());
        let count = Rc::new(Cell::new(0u32));
        let count_clone = count.clone();
        let _sub = state.on_focus_change(move |_| {
            count_clone.set(count_clone.get() + 1);
        });

        state.request_focus(id);
        assert_eq!(count.get(), 1);

        state.release_focus();
        assert_eq!(count.get(), 2);
    }

    #[test]
    fn focus_change_contains_lost_and_gained() {
        let mut sm = make_slot_map();
        let mut state = FocusState::new();
        let id_a = sm.insert(());
        let id_b = sm.insert(());
        let last_change = Rc::new(Cell::new(None::<FocusChange>));

        let lc = last_change.clone();
        let _sub = state.on_focus_change(move |change| {
            lc.set(Some(*change));
        });

        state.request_focus(id_a);
        let change = last_change.get().unwrap();
        assert_eq!(change.lost, None);
        assert_eq!(change.gained, Some(id_a));

        state.request_focus(id_b);
        let change = last_change.get().unwrap();
        assert_eq!(change.lost, Some(id_a));
        assert_eq!(change.gained, Some(id_b));

        state.release_focus();
        let change = last_change.get().unwrap();
        assert_eq!(change.lost, Some(id_b));
        assert_eq!(change.gained, None);
    }

    #[test]
    fn request_same_focus_is_noop() {
        let mut sm = make_slot_map();
        let mut state = FocusState::new();
        let id = sm.insert(());
        let count = Rc::new(Cell::new(0u32));
        let count_clone = count.clone();
        let _sub = state.on_focus_change(move |_| {
            count_clone.set(count_clone.get() + 1);
        });

        state.request_focus(id);
        assert_eq!(count.get(), 1);

        state.request_focus(id);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn push_scope_restricts_focus() {
        let mut sm = make_slot_map();
        let mut state = FocusState::new();
        let outside = sm.insert(());
        let inside_a = sm.insert(());
        let inside_b = sm.insert(());

        state.request_focus(outside);
        assert_eq!(state.focused(), Some(outside));

        state.push_scope(vec![inside_a, inside_b]);
        assert_eq!(state.focused(), None);

        state.request_focus(inside_a);
        assert_eq!(state.focused(), Some(inside_a));

        state.request_focus(outside);
        assert_eq!(state.focused(), Some(inside_a));
    }

    #[test]
    fn pop_scope_restores_previous_focus() {
        let mut sm = make_slot_map();
        let mut state = FocusState::new();
        let outside = sm.insert(());
        let inside = sm.insert(());

        state.request_focus(outside);
        state.push_scope(vec![inside]);
        state.request_focus(inside);
        state.pop_scope();

        assert_eq!(state.focused(), Some(outside));
    }
}
