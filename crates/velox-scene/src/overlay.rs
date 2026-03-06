use crate::geometry::Point;
use crate::node::NodeId;
use crate::tree::NodeTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayId(u64);

struct OverlayEntry {
    id: OverlayId,
    tree: NodeTree,
}

#[derive(Default)]
pub struct OverlayStack {
    overlays: Vec<OverlayEntry>,
    next_id: u64,
}

impl OverlayStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_overlay(&mut self) -> OverlayId {
        let id = OverlayId(self.next_id);
        self.next_id += 1;
        self.overlays.push(OverlayEntry {
            id,
            tree: NodeTree::new(),
        });
        id
    }

    pub fn pop_overlay(&mut self, id: OverlayId) -> bool {
        let Some(pos) = self.overlays.iter().position(|e| e.id == id) else {
            return false;
        };
        self.overlays.remove(pos);
        true
    }

    pub fn overlay_tree(&self, id: OverlayId) -> Option<&NodeTree> {
        self.overlays.iter().find(|e| e.id == id).map(|e| &e.tree)
    }

    pub fn overlay_tree_mut(&mut self, id: OverlayId) -> Option<&mut NodeTree> {
        self.overlays
            .iter_mut()
            .find(|e| e.id == id)
            .map(|e| &mut e.tree)
    }

    pub fn dismiss_all(&mut self) {
        self.overlays.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.overlays.is_empty()
    }

    pub fn len(&self) -> usize {
        self.overlays.len()
    }

    pub fn ids(&self) -> Vec<OverlayId> {
        self.overlays.iter().map(|e| e.id).collect()
    }

    pub fn hit_test(&self, point: Point) -> Option<(OverlayId, NodeId)> {
        for entry in self.overlays.iter().rev() {
            if let Some(node_id) = entry.tree.hit_test(point) {
                return Some((entry.id, node_id));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    #[test]
    fn push_and_pop_overlay() {
        let mut stack = OverlayStack::new();
        let id = stack.push_overlay();
        assert_eq!(stack.len(), 1);
        assert!(!stack.is_empty());

        assert!(stack.pop_overlay(id));
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn access_overlay_tree() {
        let mut stack = OverlayStack::new();
        let id = stack.push_overlay();

        let tree = stack.overlay_tree_mut(id).unwrap();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));

        let tree = stack.overlay_tree(id).unwrap();
        assert_eq!(tree.rect(root), Some(Rect::new(0.0, 0.0, 100.0, 100.0)));
    }

    #[test]
    fn dismiss_all() {
        let mut stack = OverlayStack::new();
        stack.push_overlay();
        stack.push_overlay();
        stack.push_overlay();
        assert_eq!(stack.len(), 3);

        stack.dismiss_all();
        assert!(stack.is_empty());
    }

    #[test]
    fn hit_test_checks_topmost_first() {
        let mut stack = OverlayStack::new();

        let id_bottom = stack.push_overlay();
        let tree = stack.overlay_tree_mut(id_bottom).unwrap();
        let root_bottom = tree.insert(None);
        tree.set_rect(root_bottom, Rect::new(0.0, 0.0, 200.0, 200.0));

        let id_top = stack.push_overlay();
        let tree = stack.overlay_tree_mut(id_top).unwrap();
        let root_top = tree.insert(None);
        tree.set_rect(root_top, Rect::new(0.0, 0.0, 200.0, 200.0));

        let result = stack.hit_test(Point::new(50.0, 50.0));
        assert_eq!(result, Some((id_top, root_top)));
    }

    #[test]
    fn hit_test_falls_through_to_lower_overlay() {
        let mut stack = OverlayStack::new();

        let id_bottom = stack.push_overlay();
        let tree = stack.overlay_tree_mut(id_bottom).unwrap();
        let root_bottom = tree.insert(None);
        tree.set_rect(root_bottom, Rect::new(0.0, 0.0, 200.0, 200.0));

        let id_top = stack.push_overlay();
        let tree = stack.overlay_tree_mut(id_top).unwrap();
        let root_top = tree.insert(None);
        tree.set_rect(root_top, Rect::new(0.0, 0.0, 50.0, 50.0));

        let result = stack.hit_test(Point::new(100.0, 100.0));
        assert_eq!(result, Some((id_bottom, root_bottom)));

        let _ = id_top;
        let _ = root_top;
    }

    #[test]
    fn hit_test_returns_none_when_empty() {
        let stack = OverlayStack::new();
        assert_eq!(stack.hit_test(Point::new(50.0, 50.0)), None);
    }
}
