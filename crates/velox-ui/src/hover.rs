use velox_scene::NodeId;

#[derive(Debug, PartialEq, Eq)]
pub struct HoverChange {
    pub entered: Option<NodeId>,
    pub exited: Option<NodeId>,
}

pub struct HoverManager {
    current: Option<NodeId>,
}

impl Default for HoverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HoverManager {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn hovered(&self) -> Option<NodeId> {
        self.current
    }

    pub fn set_hovered(&mut self, node: Option<NodeId>) -> HoverChange {
        if self.current == node {
            return HoverChange {
                entered: None,
                exited: None,
            };
        }
        let exited = self.current;
        let entered = node;
        self.current = node;
        HoverChange { entered, exited }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn make_ids(count: usize) -> Vec<NodeId> {
        let mut sm: SlotMap<NodeId, ()> = SlotMap::with_key();
        (0..count).map(|_| sm.insert(())).collect()
    }

    #[test]
    fn initial_state_is_none() {
        let mgr = HoverManager::new();
        assert_eq!(mgr.hovered(), None);
    }

    #[test]
    fn set_hovered_returns_changes() {
        let mut mgr = HoverManager::new();
        let ids = make_ids(1);
        let change = mgr.set_hovered(Some(ids[0]));
        assert_eq!(change.entered, Some(ids[0]));
        assert_eq!(change.exited, None);
        assert_eq!(mgr.hovered(), Some(ids[0]));
    }

    #[test]
    fn changing_hover_returns_both() {
        let mut mgr = HoverManager::new();
        let ids = make_ids(2);
        mgr.set_hovered(Some(ids[0]));
        let change = mgr.set_hovered(Some(ids[1]));
        assert_eq!(change.exited, Some(ids[0]));
        assert_eq!(change.entered, Some(ids[1]));
    }

    #[test]
    fn clear_hover() {
        let mut mgr = HoverManager::new();
        let ids = make_ids(1);
        mgr.set_hovered(Some(ids[0]));
        let change = mgr.set_hovered(None);
        assert_eq!(change.exited, Some(ids[0]));
        assert_eq!(change.entered, None);
        assert_eq!(mgr.hovered(), None);
    }

    #[test]
    fn same_hover_no_change() {
        let mut mgr = HoverManager::new();
        let ids = make_ids(1);
        mgr.set_hovered(Some(ids[0]));
        let change = mgr.set_hovered(Some(ids[0]));
        assert_eq!(change.entered, None);
        assert_eq!(change.exited, None);
    }

    #[test]
    fn none_to_none_no_change() {
        let mut mgr = HoverManager::new();
        let change = mgr.set_hovered(None);
        assert_eq!(change.entered, None);
        assert_eq!(change.exited, None);
    }
}
