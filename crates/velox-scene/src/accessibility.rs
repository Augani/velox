use crate::{NodeId, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    Window,
    Group,
    Label,
    Button,
    TextInput,
    Checkbox,
    List,
    ListItem,
    Image,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityNode {
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub disabled: bool,
}

impl AccessibilityNode {
    pub fn new(role: AccessibilityRole) -> Self {
        Self {
            role,
            label: None,
            value: None,
            disabled: false,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityTreeNode {
    pub id: NodeId,
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub disabled: bool,
    pub rect: Rect,
    pub focused: bool,
    pub children: Vec<AccessibilityTreeNode>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilityTreeSnapshot {
    pub roots: Vec<AccessibilityTreeNode>,
}

impl AccessibilityTreeSnapshot {
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn node_count(&self) -> usize {
        self.roots.iter().map(count_nodes).sum()
    }
}

fn count_nodes(node: &AccessibilityTreeNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn accessibility_node_builder() {
        let node = AccessibilityNode::new(AccessibilityRole::Button)
            .label("Send")
            .value("ready")
            .disabled(true);

        assert_eq!(node.role, AccessibilityRole::Button);
        assert_eq!(node.label.as_deref(), Some("Send"));
        assert_eq!(node.value.as_deref(), Some("ready"));
        assert!(node.disabled);
    }

    #[test]
    fn snapshot_counts_tree_nodes() {
        let mut map = SlotMap::with_key();
        let root_id = map.insert(());
        let child_id = map.insert(());

        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: root_id,
                role: AccessibilityRole::Window,
                label: None,
                value: None,
                disabled: false,
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                focused: false,
                children: vec![AccessibilityTreeNode {
                    id: child_id,
                    role: AccessibilityRole::Button,
                    label: Some("Click".into()),
                    value: None,
                    disabled: false,
                    rect: Rect::new(0.0, 0.0, 50.0, 20.0),
                    focused: true,
                    children: Vec::new(),
                }],
            }],
        };

        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.node_count(), 2);
    }
}
