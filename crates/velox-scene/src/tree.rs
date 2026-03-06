use slotmap::SlotMap;

use crate::geometry::Rect;
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
        }
    }
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

    pub(crate) fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id)
    }

    pub(crate) fn get_mut(&mut self, id: NodeId) -> Option<&mut NodeData> {
        self.nodes.get_mut(id)
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
}
