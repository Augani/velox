use velox_scene::NodeId;

pub struct ActiveManager {
    current: Option<NodeId>,
}

impl Default for ActiveManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ActiveManager {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn active(&self) -> Option<NodeId> {
        self.current
    }

    pub fn set_active(&mut self, node: Option<NodeId>) {
        self.current = node;
    }

    pub fn clear(&mut self) {
        self.current = None;
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
    fn initial_state_none() {
        let mgr = ActiveManager::new();
        assert_eq!(mgr.active(), None);
    }

    #[test]
    fn set_and_clear() {
        let mut mgr = ActiveManager::new();
        let ids = make_ids(1);
        mgr.set_active(Some(ids[0]));
        assert_eq!(mgr.active(), Some(ids[0]));
        mgr.clear();
        assert_eq!(mgr.active(), None);
    }

    #[test]
    fn set_replaces_previous() {
        let mut mgr = ActiveManager::new();
        let ids = make_ids(2);
        mgr.set_active(Some(ids[0]));
        mgr.set_active(Some(ids[1]));
        assert_eq!(mgr.active(), Some(ids[1]));
    }

    #[test]
    fn set_none_clears() {
        let mut mgr = ActiveManager::new();
        let ids = make_ids(1);
        mgr.set_active(Some(ids[0]));
        mgr.set_active(None);
        assert_eq!(mgr.active(), None);
    }
}
