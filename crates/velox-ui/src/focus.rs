use velox_scene::NodeId;

#[derive(Debug, PartialEq, Eq)]
pub struct FocusChange {
    pub gained: Option<NodeId>,
    pub lost: Option<NodeId>,
}

struct TabStop {
    node: NodeId,
    tab_index: i32,
    insertion_order: usize,
}

pub struct TabStopMap {
    stops: Vec<TabStop>,
    counter: usize,
}

impl Default for TabStopMap {
    fn default() -> Self {
        Self::new()
    }
}

impl TabStopMap {
    pub fn new() -> Self {
        Self {
            stops: Vec::new(),
            counter: 0,
        }
    }

    pub fn clear(&mut self) {
        self.stops.clear();
        self.counter = 0;
    }

    pub fn insert(&mut self, node: NodeId, tab_index: i32) {
        self.stops.push(TabStop {
            node,
            tab_index,
            insertion_order: self.counter,
        });
        self.counter += 1;
        self.stops.sort_by(|a, b| {
            a.tab_index
                .cmp(&b.tab_index)
                .then(a.insertion_order.cmp(&b.insertion_order))
        });
    }

    pub fn next(&self, current: Option<NodeId>) -> Option<NodeId> {
        if self.stops.is_empty() {
            return None;
        }
        match current {
            None => self.stops.first().map(|s| s.node),
            Some(id) => {
                let idx = self.stops.iter().position(|s| s.node == id);
                match idx {
                    Some(i) => Some(self.stops[(i + 1) % self.stops.len()].node),
                    None => self.stops.first().map(|s| s.node),
                }
            }
        }
    }

    pub fn prev(&self, current: Option<NodeId>) -> Option<NodeId> {
        if self.stops.is_empty() {
            return None;
        }
        match current {
            None => self.stops.last().map(|s| s.node),
            Some(id) => {
                let idx = self.stops.iter().position(|s| s.node == id);
                match idx {
                    Some(0) => self.stops.last().map(|s| s.node),
                    Some(i) => Some(self.stops[i - 1].node),
                    None => self.stops.last().map(|s| s.node),
                }
            }
        }
    }

    pub fn nodes(&self) -> Vec<NodeId> {
        self.stops.iter().map(|s| s.node).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.stops.is_empty()
    }
}

pub struct FocusManager {
    focused: Option<NodeId>,
    tab_stops: TabStopMap,
    traps: Vec<Vec<NodeId>>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focused: None,
            tab_stops: TabStopMap::new(),
            traps: Vec::new(),
        }
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    pub fn request_focus(&mut self, node: NodeId) -> FocusChange {
        let lost = if self.focused != Some(node) {
            self.focused
        } else {
            None
        };
        let gained = if self.focused != Some(node) {
            Some(node)
        } else {
            None
        };
        self.focused = Some(node);
        FocusChange { gained, lost }
    }

    pub fn clear_focus(&mut self) -> FocusChange {
        let lost = self.focused;
        self.focused = None;
        FocusChange { gained: None, lost }
    }

    pub fn tab_stops(&self) -> &TabStopMap {
        &self.tab_stops
    }

    pub fn tab_stops_mut(&mut self) -> &mut TabStopMap {
        &mut self.tab_stops
    }

    pub fn push_trap(&mut self, nodes: Vec<NodeId>) {
        self.traps.push(nodes);
    }

    pub fn pop_trap(&mut self) {
        self.traps.pop();
    }

    pub fn next_focus(&self) -> Option<NodeId> {
        if let Some(trap) = self.traps.last() {
            let mut map = TabStopMap::new();
            for (i, node) in trap.iter().enumerate() {
                map.insert(*node, i as i32);
            }
            return map.next(self.focused);
        }
        self.tab_stops.next(self.focused)
    }

    pub fn prev_focus(&self) -> Option<NodeId> {
        if let Some(trap) = self.traps.last() {
            let mut map = TabStopMap::new();
            for (i, node) in trap.iter().enumerate() {
                map.insert(*node, i as i32);
            }
            return map.prev(self.focused);
        }
        self.tab_stops.prev(self.focused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_focus_is_none() {
        let mgr = FocusManager::new();
        assert_eq!(mgr.focused(), None);
    }

    #[test]
    fn request_focus() {
        let mut mgr = FocusManager::new();
        let node = NodeId::from_raw_parts(1, 1);
        let change = mgr.request_focus(node);
        assert_eq!(change.gained, Some(node));
        assert_eq!(change.lost, None);
        assert_eq!(mgr.focused(), Some(node));
    }

    #[test]
    fn request_focus_change() {
        let mut mgr = FocusManager::new();
        let a = NodeId::from_raw_parts(1, 1);
        let b = NodeId::from_raw_parts(2, 1);
        mgr.request_focus(a);
        let change = mgr.request_focus(b);
        assert_eq!(change.lost, Some(a));
        assert_eq!(change.gained, Some(b));
        assert_eq!(mgr.focused(), Some(b));
    }

    #[test]
    fn request_focus_same_node_is_noop() {
        let mut mgr = FocusManager::new();
        let node = NodeId::from_raw_parts(1, 1);
        mgr.request_focus(node);
        let change = mgr.request_focus(node);
        assert_eq!(change.gained, None);
        assert_eq!(change.lost, None);
    }

    #[test]
    fn clear_focus() {
        let mut mgr = FocusManager::new();
        let node = NodeId::from_raw_parts(1, 1);
        mgr.request_focus(node);
        let change = mgr.clear_focus();
        assert_eq!(change.lost, Some(node));
        assert_eq!(change.gained, None);
        assert_eq!(mgr.focused(), None);
    }

    #[test]
    fn tab_stop_forward() {
        let mut map = TabStopMap::new();
        let a = NodeId::from_raw_parts(1, 1);
        let b = NodeId::from_raw_parts(2, 1);
        let c = NodeId::from_raw_parts(3, 1);
        map.insert(a, 0);
        map.insert(b, 0);
        map.insert(c, 0);
        assert_eq!(map.next(None), Some(a));
        assert_eq!(map.next(Some(a)), Some(b));
        assert_eq!(map.next(Some(c)), Some(a));
    }

    #[test]
    fn tab_stop_backward() {
        let mut map = TabStopMap::new();
        let a = NodeId::from_raw_parts(1, 1);
        let b = NodeId::from_raw_parts(2, 1);
        map.insert(a, 0);
        map.insert(b, 0);
        assert_eq!(map.prev(Some(a)), Some(b));
        assert_eq!(map.prev(Some(b)), Some(a));
    }

    #[test]
    fn tab_index_ordering() {
        let mut map = TabStopMap::new();
        let a = NodeId::from_raw_parts(1, 1);
        let b = NodeId::from_raw_parts(2, 1);
        let c = NodeId::from_raw_parts(3, 1);
        map.insert(a, 2);
        map.insert(b, 0);
        map.insert(c, 1);
        assert_eq!(map.next(None), Some(b));
        assert_eq!(map.next(Some(b)), Some(c));
        assert_eq!(map.next(Some(c)), Some(a));
    }

    #[test]
    fn tab_stop_empty_returns_none() {
        let map = TabStopMap::new();
        assert_eq!(map.next(None), None);
        assert_eq!(map.prev(None), None);
    }

    #[test]
    fn focus_trap_constrains_tab() {
        let mut mgr = FocusManager::new();
        let outside = NodeId::from_raw_parts(1, 1);
        let inside_a = NodeId::from_raw_parts(2, 1);
        let inside_b = NodeId::from_raw_parts(3, 1);
        mgr.tab_stops_mut().insert(outside, 0);
        mgr.push_trap(vec![inside_a, inside_b]);
        assert_eq!(mgr.next_focus(), Some(inside_a));
        mgr.request_focus(inside_b);
        assert_eq!(mgr.next_focus(), Some(inside_a));
    }

    #[test]
    fn focus_trap_pop_restores_normal_tab() {
        let mut mgr = FocusManager::new();
        let a = NodeId::from_raw_parts(1, 1);
        let b = NodeId::from_raw_parts(2, 1);
        let trap_node = NodeId::from_raw_parts(3, 1);
        mgr.tab_stops_mut().insert(a, 0);
        mgr.tab_stops_mut().insert(b, 0);
        mgr.push_trap(vec![trap_node]);
        assert_eq!(mgr.next_focus(), Some(trap_node));
        mgr.pop_trap();
        assert_eq!(mgr.next_focus(), Some(a));
    }

    #[test]
    fn tab_stop_clear() {
        let mut map = TabStopMap::new();
        let a = NodeId::from_raw_parts(1, 1);
        map.insert(a, 0);
        assert!(!map.is_empty());
        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.next(None), None);
    }
}
