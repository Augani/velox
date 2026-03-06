use std::collections::HashMap;

use velox_scene::{AccessibilityRole, AccessibilityTreeNode, AccessibilityTreeSnapshot, NodeId};

pub fn map_role(role: AccessibilityRole) -> accesskit::Role {
    match role {
        AccessibilityRole::Window => accesskit::Role::Window,
        AccessibilityRole::Group => accesskit::Role::Group,
        AccessibilityRole::Label => accesskit::Role::Label,
        AccessibilityRole::Button => accesskit::Role::Button,
        AccessibilityRole::TextInput => accesskit::Role::TextInput,
        AccessibilityRole::Checkbox => accesskit::Role::CheckBox,
        AccessibilityRole::List => accesskit::Role::List,
        AccessibilityRole::ListItem => accesskit::Role::ListItem,
        AccessibilityRole::Image => accesskit::Role::Image,
        AccessibilityRole::Custom => accesskit::Role::Unknown,
    }
}

pub struct TreeUpdateResult {
    pub nodes: Vec<(accesskit::NodeId, accesskit::Node)>,
    pub focus: Option<accesskit::NodeId>,
}

const VIRTUAL_ROOT_ID: u64 = 1;

pub struct AccessibilityBridge {
    node_id_map: HashMap<NodeId, accesskit::NodeId>,
    next_accesskit_id: u64,
}

impl AccessibilityBridge {
    pub fn new() -> Self {
        Self {
            node_id_map: HashMap::new(),
            next_accesskit_id: VIRTUAL_ROOT_ID + 1,
        }
    }

    pub fn build_tree_update(&self, snapshot: &AccessibilityTreeSnapshot) -> TreeUpdateResult {
        let mut bridge = Self::new();
        let mut nodes = Vec::new();
        let mut focus = None;

        let virtual_root_id = accesskit::NodeId(VIRTUAL_ROOT_ID);
        let mut root_children = Vec::new();

        for tree_node in &snapshot.roots {
            let ak_id = bridge.alloc_id(tree_node.id);
            root_children.push(ak_id);
            bridge.convert_node(tree_node, &mut nodes, &mut focus);
        }

        let mut virtual_root = accesskit::Node::new(accesskit::Role::Window);
        virtual_root.set_children(root_children);
        nodes.insert(0, (virtual_root_id, virtual_root));

        TreeUpdateResult { nodes, focus }
    }

    fn alloc_id(&mut self, node_id: NodeId) -> accesskit::NodeId {
        *self.node_id_map.entry(node_id).or_insert_with(|| {
            let id = accesskit::NodeId(self.next_accesskit_id);
            self.next_accesskit_id += 1;
            id
        })
    }

    fn convert_node(
        &mut self,
        tree_node: &AccessibilityTreeNode,
        nodes: &mut Vec<(accesskit::NodeId, accesskit::Node)>,
        focus: &mut Option<accesskit::NodeId>,
    ) {
        let ak_id = self.alloc_id(tree_node.id);
        let mut node = accesskit::Node::new(map_role(tree_node.role));

        if let Some(label) = &tree_node.label {
            node.set_label(label.clone());
        }
        if let Some(value) = &tree_node.value {
            node.set_value(value.clone());
        }
        if tree_node.disabled {
            node.set_disabled();
        }

        let bounds = accesskit::Rect {
            x0: tree_node.rect.x as f64,
            y0: tree_node.rect.y as f64,
            x1: (tree_node.rect.x + tree_node.rect.width) as f64,
            y1: (tree_node.rect.y + tree_node.rect.height) as f64,
        };
        node.set_bounds(bounds);

        if tree_node.focused {
            *focus = Some(ak_id);
        }

        let child_ids: Vec<accesskit::NodeId> = tree_node
            .children
            .iter()
            .map(|c| self.alloc_id(c.id))
            .collect();
        if !child_ids.is_empty() {
            node.set_children(child_ids);
        }

        nodes.push((ak_id, node));

        for child in &tree_node.children {
            self.convert_node(child, nodes, focus);
        }
    }
}

impl Default for AccessibilityBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_scene::Rect;

    #[test]
    fn map_button_role() {
        assert_eq!(map_role(AccessibilityRole::Button), accesskit::Role::Button);
    }

    #[test]
    fn map_text_input_role() {
        assert_eq!(
            map_role(AccessibilityRole::TextInput),
            accesskit::Role::TextInput
        );
    }

    #[test]
    fn map_custom_role() {
        assert_eq!(
            map_role(AccessibilityRole::Custom),
            accesskit::Role::Unknown
        );
    }

    #[test]
    fn convert_single_node_snapshot() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Button,
                label: Some("OK".into()),
                value: None,
                disabled: false,
                rect: Rect::new(10.0, 20.0, 100.0, 40.0),
                focused: true,
                children: Vec::new(),
            }],
        };
        let bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        assert_eq!(update.nodes.len(), 2);
        assert!(update.focus.is_some());
    }

    #[test]
    fn convert_empty_snapshot() {
        let snapshot = AccessibilityTreeSnapshot::default();
        let bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        assert_eq!(update.nodes.len(), 1);
        assert!(update.focus.is_none());
    }
}
