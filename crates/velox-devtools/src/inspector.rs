use velox_scene::{AccessibilityRole, AccessibilityTreeSnapshot, NodeId, Point, Rect};

#[derive(Debug, Clone)]
pub struct InspectorNode {
    pub id: NodeId,
    pub role: Option<AccessibilityRole>,
    pub label: Option<String>,
    pub rect: Rect,
    pub focused: bool,
    pub children: Vec<InspectorNode>,
}

#[derive(Debug, Clone)]
pub struct InspectorSnapshot {
    pub roots: Vec<InspectorNode>,
}

impl InspectorSnapshot {
    pub fn from_accessibility_tree(tree: &AccessibilityTreeSnapshot) -> Self {
        Self {
            roots: tree.roots.iter().map(convert_a11y_node).collect(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.roots.iter().map(count_nodes).sum()
    }

    pub fn find_at(&self, point: Point) -> Option<&InspectorNode> {
        for root in self.roots.iter().rev() {
            if let Some(hit) = find_at_recursive(root, point) {
                return Some(hit);
            }
        }
        None
    }

    pub fn format_tree(&self) -> String {
        let mut out = String::new();
        for root in &self.roots {
            format_node(&mut out, root, 0);
        }
        out
    }
}

fn convert_a11y_node(node: &velox_scene::AccessibilityTreeNode) -> InspectorNode {
    InspectorNode {
        id: node.id,
        role: Some(node.role),
        label: node.label.clone(),
        rect: node.rect,
        focused: node.focused,
        children: node.children.iter().map(convert_a11y_node).collect(),
    }
}

fn count_nodes(node: &InspectorNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn find_at_recursive(node: &InspectorNode, point: Point) -> Option<&InspectorNode> {
    if !node.rect.contains(point) {
        return None;
    }
    for child in node.children.iter().rev() {
        if let Some(hit) = find_at_recursive(child, point) {
            return Some(hit);
        }
    }
    Some(node)
}

fn format_node(out: &mut String, node: &InspectorNode, depth: usize) {
    use std::fmt::Write;
    let indent = "  ".repeat(depth);
    let role_str = match node.role {
        Some(role) => format!("{role:?}"),
        None => "Node".to_string(),
    };
    let label_str = match &node.label {
        Some(label) => format!(" \"{label}\""),
        None => String::new(),
    };
    let focus_str = if node.focused { " [focused]" } else { "" };
    let _ = writeln!(
        out,
        "{indent}{role_str}{label_str} ({:.0},{:.0} {:.0}x{:.0}){focus_str}",
        node.rect.x, node.rect.y, node.rect.width, node.rect.height,
    );
    for child in &node.children {
        format_node(out, child, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_scene::{AccessibilityRole, AccessibilityTreeNode, AccessibilityTreeSnapshot, Rect};

    fn make_snapshot() -> AccessibilityTreeSnapshot {
        use slotmap::SlotMap;
        let mut map: SlotMap<NodeId, ()> = SlotMap::with_key();
        let root_id = map.insert(());
        let child_id = map.insert(());

        AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: root_id,
                role: AccessibilityRole::Window,
                label: Some("Main".into()),
                value: None,
                disabled: false,
                supported_actions: Vec::new(),
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 800.0, 600.0),
                focused: false,
                children: vec![AccessibilityTreeNode {
                    id: child_id,
                    role: AccessibilityRole::Button,
                    label: Some("OK".into()),
                    value: None,
                    disabled: false,
                    supported_actions: Vec::new(),
                    text_selection: None,
                    text_runs: Vec::new(),
                    rect: Rect::new(10.0, 10.0, 80.0, 30.0),
                    focused: true,
                    children: Vec::new(),
                }],
            }],
        }
    }

    #[test]
    fn from_accessibility_tree() {
        let snapshot = InspectorSnapshot::from_accessibility_tree(&make_snapshot());
        assert_eq!(snapshot.node_count(), 2);
    }

    #[test]
    fn find_at_hits_deepest_node() {
        let snapshot = InspectorSnapshot::from_accessibility_tree(&make_snapshot());
        let hit = snapshot.find_at(Point::new(15.0, 15.0));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().label.as_deref(), Some("OK"));
    }

    #[test]
    fn find_at_misses_outside() {
        let snapshot = InspectorSnapshot::from_accessibility_tree(&make_snapshot());
        assert!(snapshot.find_at(Point::new(900.0, 900.0)).is_none());
    }

    #[test]
    fn format_tree_produces_output() {
        let snapshot = InspectorSnapshot::from_accessibility_tree(&make_snapshot());
        let output = snapshot.format_tree();
        assert!(output.contains("Window"));
        assert!(output.contains("Button"));
        assert!(output.contains("OK"));
        assert!(output.contains("[focused]"));
    }
}
