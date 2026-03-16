use std::collections::HashMap;
use velox_scene::NodeId;

use crate::interactive::EventHandlers;
use crate::style::CursorStyle;

#[derive(Default)]
pub struct DispatchNodeData {
    pub handlers: EventHandlers,
    pub cursor: Option<CursorStyle>,
    pub key_context: Option<String>,
    pub focusable: bool,
    pub tab_index: Option<i32>,
    pub scrollable_x: bool,
    pub scrollable_y: bool,
}

pub(crate) struct DispatchNode {
    parent: Option<NodeId>,
    data: DispatchNodeData,
}

pub struct DispatchTree {
    pub(crate) nodes: HashMap<NodeId, DispatchNode>,
}

impl Default for DispatchTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DispatchTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn register(&mut self, node: NodeId, parent: Option<NodeId>, data: DispatchNodeData) {
        self.nodes.insert(node, DispatchNode { parent, data });
    }

    pub fn get(&self, node: NodeId) -> Option<&DispatchNodeData> {
        self.nodes.get(&node).map(|n| &n.data)
    }

    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut DispatchNodeData> {
        self.nodes.get_mut(&node).map(|n| &mut n.data)
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes.get(&node).and_then(|n| n.parent)
    }

    pub fn ancestors(&self, node: NodeId) -> AncestorIter<'_> {
        AncestorIter {
            tree: self,
            current: self.nodes.get(&node).and_then(|n| n.parent),
        }
    }

    pub fn capture_path(&self, target: NodeId) -> Vec<NodeId> {
        let mut path: Vec<NodeId> = self.ancestors(target).collect();
        path.reverse();
        path.push(target);
        path
    }

    pub fn bubble_path(&self, target: NodeId) -> Vec<NodeId> {
        let mut path = vec![target];
        path.extend(self.ancestors(target));
        path
    }
}

pub struct AncestorIter<'a> {
    tree: &'a DispatchTree,
    current: Option<NodeId>,
}

impl<'a> Iterator for AncestorIter<'a> {
    type Item = NodeId;
    fn next(&mut self) -> Option<NodeId> {
        let node = self.current?;
        self.current = self.tree.nodes.get(&node).and_then(|n| n.parent);
        Some(node)
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
    fn build_dispatch_tree_from_slots() {
        let tree = DispatchTree::new();
        assert!(tree.nodes.is_empty());
    }

    #[test]
    fn register_and_lookup_node() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(1);
        tree.register(ids[0], None, DispatchNodeData::default());
        assert!(tree.get(ids[0]).is_some());
    }

    #[test]
    fn ancestors_walk_to_root() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(3);
        let root = ids[0];
        let child = ids[1];
        let grandchild = ids[2];
        tree.register(root, None, DispatchNodeData::default());
        tree.register(child, Some(root), DispatchNodeData::default());
        tree.register(grandchild, Some(child), DispatchNodeData::default());
        let ancestors: Vec<_> = tree.ancestors(grandchild).collect();
        assert_eq!(ancestors, vec![child, root]);
    }

    #[test]
    fn capture_path_is_root_to_target() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(2);
        let root = ids[0];
        let child = ids[1];
        tree.register(root, None, DispatchNodeData::default());
        tree.register(child, Some(root), DispatchNodeData::default());
        let path = tree.capture_path(child);
        assert_eq!(path, vec![root, child]);
    }

    #[test]
    fn bubble_path_is_target_to_root() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(2);
        let root = ids[0];
        let child = ids[1];
        tree.register(root, None, DispatchNodeData::default());
        tree.register(child, Some(root), DispatchNodeData::default());
        let path = tree.bubble_path(child);
        assert_eq!(path, vec![child, root]);
    }

    #[test]
    fn clear_removes_all_nodes() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(2);
        tree.register(ids[0], None, DispatchNodeData::default());
        tree.register(ids[1], Some(ids[0]), DispatchNodeData::default());
        tree.clear();
        assert!(tree.nodes.is_empty());
        assert!(tree.get(ids[0]).is_none());
    }

    #[test]
    fn get_mut_modifies_data() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(1);
        tree.register(ids[0], None, DispatchNodeData::default());
        if let Some(data) = tree.get_mut(ids[0]) {
            data.focusable = true;
            data.cursor = Some(CursorStyle::Pointer);
        }
        let data = tree.get(ids[0]).unwrap();
        assert!(data.focusable);
        assert_eq!(data.cursor, Some(CursorStyle::Pointer));
    }

    #[test]
    fn parent_returns_none_for_root() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(1);
        tree.register(ids[0], None, DispatchNodeData::default());
        assert_eq!(tree.parent(ids[0]), None);
    }

    #[test]
    fn parent_returns_parent_for_child() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(2);
        tree.register(ids[0], None, DispatchNodeData::default());
        tree.register(ids[1], Some(ids[0]), DispatchNodeData::default());
        assert_eq!(tree.parent(ids[1]), Some(ids[0]));
    }

    #[test]
    fn ancestors_empty_for_root() {
        let mut tree = DispatchTree::new();
        let ids = make_ids(1);
        tree.register(ids[0], None, DispatchNodeData::default());
        let ancestors: Vec<_> = tree.ancestors(ids[0]).collect();
        assert!(ancestors.is_empty());
    }
}
