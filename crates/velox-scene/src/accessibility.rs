use crate::{NodeId, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityRole {
    Window,
    Group,
    Label,
    Button,
    TextInput,
    TextRun,
    Checkbox,
    List,
    ListItem,
    Image,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessibilityActionSupport {
    Focus,
    Blur,
    Click,
    SetValue,
    ReplaceSelectedText,
    SetTextSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessibilityTextSelection {
    pub anchor: usize,
    pub focus: usize,
}

impl AccessibilityTextSelection {
    pub fn collapsed(index: usize) -> Self {
        Self {
            anchor: index,
            focus: index,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityTextRun {
    pub text: String,
    pub byte_start: usize,
    pub rect: Rect,
}

impl AccessibilityTextRun {
    pub fn new(text: impl Into<String>, byte_start: usize, rect: Rect) -> Self {
        Self {
            text: text.into(),
            byte_start,
            rect,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessibilityAction {
    SetValue(String),
    ReplaceSelectedText(String),
    SetTextSelection(AccessibilityTextSelection),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityNode {
    pub role: AccessibilityRole,
    pub label: Option<String>,
    pub value: Option<String>,
    pub disabled: bool,
    pub supported_actions: Vec<AccessibilityActionSupport>,
    pub text_selection: Option<AccessibilityTextSelection>,
    pub text_runs: Vec<AccessibilityTextRun>,
}

impl AccessibilityNode {
    pub fn new(role: AccessibilityRole) -> Self {
        Self {
            role,
            label: None,
            value: None,
            disabled: false,
            supported_actions: Vec::new(),
            text_selection: None,
            text_runs: Vec::new(),
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

    pub fn supports_action(mut self, action: AccessibilityActionSupport) -> Self {
        if !self.supported_actions.contains(&action) {
            self.supported_actions.push(action);
        }
        self
    }

    pub fn supports_focus_actions(self) -> Self {
        self.supports_actions([
            AccessibilityActionSupport::Focus,
            AccessibilityActionSupport::Blur,
        ])
    }

    pub fn supports_click_action(self) -> Self {
        self.supports_action(AccessibilityActionSupport::Click)
    }

    pub fn supports_actions(
        mut self,
        actions: impl IntoIterator<Item = AccessibilityActionSupport>,
    ) -> Self {
        for action in actions {
            if !self.supported_actions.contains(&action) {
                self.supported_actions.push(action);
            }
        }
        self
    }

    pub fn supports_text_input_actions(self) -> Self {
        self.supports_focus_actions().supports_actions([
            AccessibilityActionSupport::SetTextSelection,
            AccessibilityActionSupport::SetValue,
            AccessibilityActionSupport::ReplaceSelectedText,
        ])
    }

    pub fn text_selection(mut self, selection: AccessibilityTextSelection) -> Self {
        self.text_selection = Some(selection);
        self
    }

    pub fn text_run(mut self, run: AccessibilityTextRun) -> Self {
        self.text_runs.push(run);
        self
    }

    pub fn text_runs(mut self, runs: Vec<AccessibilityTextRun>) -> Self {
        self.text_runs = runs;
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
    pub supported_actions: Vec<AccessibilityActionSupport>,
    pub text_selection: Option<AccessibilityTextSelection>,
    pub text_runs: Vec<AccessibilityTextRun>,
    pub rect: Rect,
    pub focused: bool,
    pub children: Vec<AccessibilityTreeNode>,
}

impl AccessibilityTreeNode {
    pub fn supports_action(&self, action: AccessibilityActionSupport) -> bool {
        !self.disabled && self.supported_actions.contains(&action)
    }

    pub fn find_node(&self, id: NodeId) -> Option<&AccessibilityTreeNode> {
        if self.id == id {
            return Some(self);
        }

        self.children.iter().find_map(|child| child.find_node(id))
    }
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

    pub fn find_node(&self, id: NodeId) -> Option<&AccessibilityTreeNode> {
        self.roots.iter().find_map(|root| root.find_node(id))
    }

    pub fn supports_action(&self, id: NodeId, action: AccessibilityActionSupport) -> bool {
        self.find_node(id)
            .is_some_and(|node| node.supports_action(action))
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
            .supports_click_action()
            .text_selection(AccessibilityTextSelection::collapsed(2))
            .text_run(AccessibilityTextRun::new(
                "Send",
                0,
                Rect::new(1.0, 2.0, 30.0, 14.0),
            ))
            .disabled(true);

        assert_eq!(node.role, AccessibilityRole::Button);
        assert_eq!(node.label.as_deref(), Some("Send"));
        assert_eq!(node.value.as_deref(), Some("ready"));
        assert!(node.disabled);
        assert_eq!(
            node.supported_actions,
            vec![AccessibilityActionSupport::Click]
        );
        assert_eq!(
            node.text_selection,
            Some(AccessibilityTextSelection::collapsed(2))
        );
        assert_eq!(node.text_runs.len(), 1);
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
                supported_actions: Vec::new(),
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                focused: false,
                children: vec![AccessibilityTreeNode {
                    id: child_id,
                    role: AccessibilityRole::Button,
                    label: Some("Click".into()),
                    value: None,
                    disabled: false,
                    supported_actions: vec![AccessibilityActionSupport::Click],
                    text_selection: None,
                    text_runs: Vec::new(),
                    rect: Rect::new(0.0, 0.0, 50.0, 20.0),
                    focused: true,
                    children: Vec::new(),
                }],
            }],
        };

        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.node_count(), 2);
    }

    #[test]
    fn text_input_helper_adds_focus_and_edit_actions() {
        let node =
            AccessibilityNode::new(AccessibilityRole::TextInput).supports_text_input_actions();

        assert_eq!(
            node.supported_actions,
            vec![
                AccessibilityActionSupport::Focus,
                AccessibilityActionSupport::Blur,
                AccessibilityActionSupport::SetTextSelection,
                AccessibilityActionSupport::SetValue,
                AccessibilityActionSupport::ReplaceSelectedText,
            ]
        );
    }

    #[test]
    fn snapshot_finds_nodes_and_checks_supported_actions() {
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
                supported_actions: Vec::new(),
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                focused: false,
                children: vec![AccessibilityTreeNode {
                    id: child_id,
                    role: AccessibilityRole::Button,
                    label: Some("Click".into()),
                    value: None,
                    disabled: false,
                    supported_actions: vec![AccessibilityActionSupport::Click],
                    text_selection: None,
                    text_runs: Vec::new(),
                    rect: Rect::new(0.0, 0.0, 50.0, 20.0),
                    focused: false,
                    children: Vec::new(),
                }],
            }],
        };

        assert!(snapshot.find_node(child_id).is_some());
        assert!(snapshot.supports_action(child_id, AccessibilityActionSupport::Click));
        assert!(!snapshot.supports_action(child_id, AccessibilityActionSupport::Focus));
    }
}
