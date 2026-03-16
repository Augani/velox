use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use accesskit::{
    Action, ActionData, ActionHandler, ActionRequest, ActivationHandler, DeactivationHandler, Tree,
    TreeUpdate,
};
use velox_scene::{
    AccessibilityActionSupport, AccessibilityRole, AccessibilityTreeNode,
    AccessibilityTreeSnapshot, NodeId,
};

pub fn map_role(role: AccessibilityRole) -> accesskit::Role {
    match role {
        AccessibilityRole::Window => accesskit::Role::Window,
        AccessibilityRole::Group => accesskit::Role::Group,
        AccessibilityRole::Label => accesskit::Role::Label,
        AccessibilityRole::Button => accesskit::Role::Button,
        AccessibilityRole::TextInput => accesskit::Role::TextInput,
        AccessibilityRole::TextRun => accesskit::Role::TextRun,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityActionKind {
    Focus,
    Blur,
    Click,
    SetValue,
    ReplaceSelectedText,
    SetTextSelection,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityActionRequest {
    pub target: NodeId,
    pub kind: AccessibilityActionKind,
    pub text: Option<String>,
    pub selection: Option<velox_scene::AccessibilityTextSelection>,
}

const VIRTUAL_ROOT_ID: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextRunKey {
    owner: NodeId,
    index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextRunData {
    owner: NodeId,
    byte_start: usize,
    byte_lengths: Vec<usize>,
}

pub struct AccessibilityBridge {
    node_id_map: HashMap<NodeId, accesskit::NodeId>,
    reverse_node_id_map: HashMap<accesskit::NodeId, NodeId>,
    text_run_id_map: HashMap<TextRunKey, accesskit::NodeId>,
    reverse_text_run_map: HashMap<accesskit::NodeId, TextRunData>,
    next_accesskit_id: u64,
}

impl AccessibilityBridge {
    pub fn new() -> Self {
        Self {
            node_id_map: HashMap::new(),
            reverse_node_id_map: HashMap::new(),
            text_run_id_map: HashMap::new(),
            reverse_text_run_map: HashMap::new(),
            next_accesskit_id: VIRTUAL_ROOT_ID + 1,
        }
    }

    pub fn build_tree_update(&mut self, snapshot: &AccessibilityTreeSnapshot) -> TreeUpdateResult {
        let mut live_nodes = HashSet::new();
        collect_snapshot_node_ids(snapshot, &mut live_nodes);
        let mut live_text_runs = HashSet::new();
        collect_snapshot_text_run_keys(snapshot, &mut live_text_runs);
        self.node_id_map
            .retain(|node_id, _| live_nodes.contains(node_id));
        self.text_run_id_map
            .retain(|key, _| live_text_runs.contains(key));
        self.rebuild_reverse_map();
        self.reverse_text_run_map.clear();

        let mut nodes = Vec::new();
        let mut focus = None;

        let virtual_root_id = accesskit::NodeId(VIRTUAL_ROOT_ID);
        let mut root_children = Vec::new();

        for tree_node in &snapshot.roots {
            let ak_id = self.alloc_id(tree_node.id);
            root_children.push(ak_id);
            self.convert_node(tree_node, &mut nodes, &mut focus);
        }

        let mut virtual_root = accesskit::Node::new(accesskit::Role::Window);
        virtual_root.set_children(root_children);
        nodes.insert(0, (virtual_root_id, virtual_root));

        TreeUpdateResult { nodes, focus }
    }

    pub fn virtual_root_id(&self) -> accesskit::NodeId {
        accesskit::NodeId(VIRTUAL_ROOT_ID)
    }

    pub fn build_accesskit_tree_update(
        &mut self,
        snapshot: &AccessibilityTreeSnapshot,
        tree: Option<Tree>,
    ) -> TreeUpdate {
        let update = self.build_tree_update(snapshot);
        TreeUpdate {
            nodes: update.nodes,
            tree,
            focus: update.focus.unwrap_or(self.virtual_root_id()),
        }
    }

    pub fn build_initial_tree_update(
        &mut self,
        snapshot: &AccessibilityTreeSnapshot,
        app_name: Option<&str>,
    ) -> TreeUpdate {
        let mut tree = Tree::new(self.virtual_root_id());
        tree.app_name = app_name.map(str::to_owned);
        tree.toolkit_name = Some(String::from("Velox"));
        tree.toolkit_version = Some(env!("CARGO_PKG_VERSION").to_owned());
        self.build_accesskit_tree_update(snapshot, Some(tree))
    }

    pub fn build_incremental_tree_update(
        &mut self,
        snapshot: &AccessibilityTreeSnapshot,
    ) -> TreeUpdate {
        self.build_accesskit_tree_update(snapshot, None)
    }

    fn alloc_id(&mut self, node_id: NodeId) -> accesskit::NodeId {
        let accesskit_id = *self.node_id_map.entry(node_id).or_insert_with(|| {
            let id = accesskit::NodeId(self.next_accesskit_id);
            self.next_accesskit_id += 1;
            id
        });
        self.reverse_node_id_map.insert(accesskit_id, node_id);
        accesskit_id
    }

    fn alloc_text_run_id(&mut self, owner: NodeId, index: usize) -> accesskit::NodeId {
        *self
            .text_run_id_map
            .entry(TextRunKey { owner, index })
            .or_insert_with(|| {
                let id = accesskit::NodeId(self.next_accesskit_id);
                self.next_accesskit_id += 1;
                id
            })
    }

    fn rebuild_reverse_map(&mut self) {
        self.reverse_node_id_map = self
            .node_id_map
            .iter()
            .map(|(node_id, accesskit_id)| (*accesskit_id, *node_id))
            .collect();
    }

    fn scene_node_for_accesskit(&self, accesskit_id: accesskit::NodeId) -> Option<NodeId> {
        self.reverse_node_id_map.get(&accesskit_id).copied()
    }

    fn text_run_owner_for_accesskit(&self, accesskit_id: accesskit::NodeId) -> Option<NodeId> {
        self.reverse_text_run_map
            .get(&accesskit_id)
            .map(|data| data.owner)
    }

    fn resolve_target_node(&self, accesskit_id: accesskit::NodeId) -> Option<NodeId> {
        self.scene_node_for_accesskit(accesskit_id)
            .or_else(|| self.text_run_owner_for_accesskit(accesskit_id))
    }

    fn decode_text_selection(
        &self,
        target: NodeId,
        selection: &accesskit::TextSelection,
    ) -> Option<velox_scene::AccessibilityTextSelection> {
        Some(velox_scene::AccessibilityTextSelection {
            anchor: self.decode_text_position(target, selection.anchor)?,
            focus: self.decode_text_position(target, selection.focus)?,
        })
    }

    fn decode_text_position(
        &self,
        target: NodeId,
        position: accesskit::TextPosition,
    ) -> Option<usize> {
        let data = self.reverse_text_run_map.get(&position.node)?;
        if data.owner != target {
            return None;
        }
        Some(
            data.byte_start
                + character_index_to_byte_offset(&data.byte_lengths, position.character_index),
        )
    }

    fn convert_node(
        &mut self,
        tree_node: &AccessibilityTreeNode,
        nodes: &mut Vec<(accesskit::NodeId, accesskit::Node)>,
        focus: &mut Option<accesskit::NodeId>,
    ) {
        let ak_id = self.alloc_id(tree_node.id);
        let mut node = accesskit::Node::new(map_role(tree_node.role));
        let text_runs = text_runs_for_node(tree_node);
        let resolved_text_runs = text_runs
            .iter()
            .enumerate()
            .map(|(index, run)| ResolvedTextRun {
                id: self.alloc_text_run_id(tree_node.id, index),
                rect: run.rect,
                byte_start: run.byte_start,
                byte_lengths: utf8_character_lengths(&run.text),
                text: run.text.clone(),
            })
            .collect::<Vec<_>>();

        if let Some(label) = &tree_node.label {
            node.set_label(label.clone());
        }
        if let Some(value) = &tree_node.value {
            node.set_value(value.clone());
        }
        if tree_node.disabled {
            node.set_disabled();
        }
        add_supported_actions(&mut node, &tree_node.supported_actions, tree_node.disabled);

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

        if !resolved_text_runs.is_empty()
            && let Some(selection) = tree_node.text_selection
            && let (Some(anchor), Some(focus_position)) = (
                encode_text_position(&resolved_text_runs, selection.anchor),
                encode_text_position(&resolved_text_runs, selection.focus),
            )
        {
            node.set_text_selection(accesskit::TextSelection {
                anchor,
                focus: focus_position,
            });
        }

        let mut child_ids: Vec<_> = resolved_text_runs.iter().map(|run| run.id).collect();
        child_ids.extend(tree_node.children.iter().map(|c| self.alloc_id(c.id)));
        if !child_ids.is_empty() {
            node.set_children(child_ids);
        }

        nodes.push((ak_id, node));

        for run in resolved_text_runs {
            let mut run_node = accesskit::Node::new(accesskit::Role::TextRun);
            run_node.set_value(run.text);
            run_node.set_character_lengths(
                run.byte_lengths
                    .iter()
                    .map(|length| *length as u8)
                    .collect::<Vec<_>>(),
            );
            run_node.set_bounds(rect_to_accesskit(run.rect));
            self.reverse_text_run_map.insert(
                run.id,
                TextRunData {
                    owner: tree_node.id,
                    byte_start: run.byte_start,
                    byte_lengths: run.byte_lengths,
                },
            );
            nodes.push((run.id, run_node));
        }

        for child in &tree_node.children {
            self.convert_node(child, nodes, focus);
        }
    }
}

struct ResolvedTextRun {
    id: accesskit::NodeId,
    rect: velox_scene::Rect,
    byte_start: usize,
    byte_lengths: Vec<usize>,
    text: String,
}

fn collect_snapshot_node_ids(snapshot: &AccessibilityTreeSnapshot, ids: &mut HashSet<NodeId>) {
    for root in &snapshot.roots {
        collect_tree_node_ids(root, ids);
    }
}

fn collect_tree_node_ids(node: &AccessibilityTreeNode, ids: &mut HashSet<NodeId>) {
    ids.insert(node.id);
    for child in &node.children {
        collect_tree_node_ids(child, ids);
    }
}

fn collect_snapshot_text_run_keys(
    snapshot: &AccessibilityTreeSnapshot,
    ids: &mut HashSet<TextRunKey>,
) {
    for root in &snapshot.roots {
        collect_text_run_keys(root, ids);
    }
}

fn collect_text_run_keys(node: &AccessibilityTreeNode, ids: &mut HashSet<TextRunKey>) {
    for index in 0..text_runs_for_node(node).len() {
        ids.insert(TextRunKey {
            owner: node.id,
            index,
        });
    }
    for child in &node.children {
        collect_text_run_keys(child, ids);
    }
}

fn text_runs_for_node(node: &AccessibilityTreeNode) -> Vec<velox_scene::AccessibilityTextRun> {
    if !node.text_runs.is_empty() {
        node.text_runs.clone()
    } else {
        match node.role {
            AccessibilityRole::TextInput => vec![velox_scene::AccessibilityTextRun::new(
                node.value.as_deref().unwrap_or(""),
                0,
                node.rect,
            )],
            _ => Vec::new(),
        }
    }
}

fn add_supported_actions(
    node: &mut accesskit::Node,
    supported_actions: &[AccessibilityActionSupport],
    disabled: bool,
) {
    if disabled {
        return;
    }

    for action in supported_actions {
        let accesskit_action = match action {
            AccessibilityActionSupport::Focus => Action::Focus,
            AccessibilityActionSupport::Blur => Action::Blur,
            AccessibilityActionSupport::Click => Action::Click,
            AccessibilityActionSupport::SetValue => Action::SetValue,
            AccessibilityActionSupport::ReplaceSelectedText => Action::ReplaceSelectedText,
            AccessibilityActionSupport::SetTextSelection => Action::SetTextSelection,
        };
        if !node.supports_action(accesskit_action) {
            node.add_action(accesskit_action);
        }
    }
}

fn supported_action_for_request(action: Action) -> Option<AccessibilityActionSupport> {
    match action {
        Action::Focus => Some(AccessibilityActionSupport::Focus),
        Action::Blur => Some(AccessibilityActionSupport::Blur),
        Action::Click => Some(AccessibilityActionSupport::Click),
        Action::SetValue => Some(AccessibilityActionSupport::SetValue),
        Action::ReplaceSelectedText => Some(AccessibilityActionSupport::ReplaceSelectedText),
        Action::SetTextSelection => Some(AccessibilityActionSupport::SetTextSelection),
        _ => None,
    }
}

fn decode_action_request(
    bridge: &AccessibilityBridge,
    target: NodeId,
    request: ActionRequest,
) -> Option<AccessibilityActionRequest> {
    match (request.action, request.data) {
        (Action::Focus, _) => Some(AccessibilityActionRequest {
            target,
            kind: AccessibilityActionKind::Focus,
            text: None,
            selection: None,
        }),
        (Action::Blur, _) => Some(AccessibilityActionRequest {
            target,
            kind: AccessibilityActionKind::Blur,
            text: None,
            selection: None,
        }),
        (Action::Click, _) => Some(AccessibilityActionRequest {
            target,
            kind: AccessibilityActionKind::Click,
            text: None,
            selection: None,
        }),
        (Action::SetValue, Some(ActionData::Value(value))) => Some(AccessibilityActionRequest {
            target,
            kind: AccessibilityActionKind::SetValue,
            text: Some(value.into()),
            selection: None,
        }),
        (Action::ReplaceSelectedText, Some(ActionData::Value(value))) => {
            Some(AccessibilityActionRequest {
                target,
                kind: AccessibilityActionKind::ReplaceSelectedText,
                text: Some(value.into()),
                selection: None,
            })
        }
        (Action::SetTextSelection, Some(ActionData::SetTextSelection(selection))) => {
            Some(AccessibilityActionRequest {
                target,
                kind: AccessibilityActionKind::SetTextSelection,
                text: None,
                selection: bridge.decode_text_selection(target, &selection),
            })
        }
        _ => None,
    }
}

fn utf8_character_lengths(text: &str) -> Vec<usize> {
    text.chars().map(|ch| ch.len_utf8()).collect()
}

fn rect_to_accesskit(rect: velox_scene::Rect) -> accesskit::Rect {
    accesskit::Rect {
        x0: rect.x as f64,
        y0: rect.y as f64,
        x1: (rect.x + rect.width) as f64,
        y1: (rect.y + rect.height) as f64,
    }
}

fn encode_text_position(
    runs: &[ResolvedTextRun],
    byte_offset: usize,
) -> Option<accesskit::TextPosition> {
    let first = runs.first()?;
    let total_end = runs
        .iter()
        .map(|run| run.byte_start + run.byte_lengths.iter().sum::<usize>())
        .max()
        .unwrap_or(first.byte_start);
    let clamped = byte_offset.min(total_end);

    for (index, run) in runs.iter().enumerate() {
        let run_len: usize = run.byte_lengths.iter().sum();
        let run_end = run.byte_start + run_len;
        let next_start = runs
            .get(index + 1)
            .map(|next| next.byte_start)
            .unwrap_or(run_end);

        if clamped < next_start || index + 1 == runs.len() {
            let relative_offset = clamped.saturating_sub(run.byte_start).min(run_len);
            return Some(accesskit::TextPosition {
                node: run.id,
                character_index: byte_offset_to_character_index(&run.byte_lengths, relative_offset),
            });
        }
    }

    Some(accesskit::TextPosition {
        node: first.id,
        character_index: 0,
    })
}

fn byte_offset_to_character_index(byte_lengths: &[usize], byte_offset: usize) -> usize {
    let max_offset: usize = byte_lengths.iter().sum();
    let clamped = byte_offset.min(max_offset);
    let mut total = 0usize;
    let mut index = 0usize;
    for length in byte_lengths {
        if total >= clamped {
            break;
        }
        total += *length;
        index += 1;
    }
    index
}

fn character_index_to_byte_offset(byte_lengths: &[usize], character_index: usize) -> usize {
    byte_lengths
        .iter()
        .take(character_index.min(byte_lengths.len()))
        .sum()
}

impl Default for AccessibilityBridge {
    fn default() -> Self {
        Self::new()
    }
}

struct AccessibilityAdapterState {
    bridge: AccessibilityBridge,
    snapshot: AccessibilityTreeSnapshot,
    app_name: Option<String>,
    pending_actions: Vec<AccessibilityActionRequest>,
}

impl AccessibilityAdapterState {
    fn new(app_name: Option<String>) -> Self {
        Self {
            bridge: AccessibilityBridge::new(),
            snapshot: AccessibilityTreeSnapshot::default(),
            app_name,
            pending_actions: Vec::new(),
        }
    }

    fn initial_tree_update(&mut self) -> TreeUpdate {
        self.bridge
            .build_initial_tree_update(&self.snapshot, self.app_name.as_deref())
    }

    fn incremental_tree_update(&mut self) -> TreeUpdate {
        self.bridge.build_incremental_tree_update(&self.snapshot)
    }

    fn drain_pending_actions(&mut self) -> Vec<AccessibilityActionRequest> {
        std::mem::take(&mut self.pending_actions)
    }
}

#[derive(Clone)]
struct SharedAccessibilityState(Arc<Mutex<AccessibilityAdapterState>>);

impl ActivationHandler for SharedAccessibilityState {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        Some(
            self.0
                .lock()
                .expect("accessibility state lock poisoned")
                .initial_tree_update(),
        )
    }
}

impl ActionHandler for SharedAccessibilityState {
    fn do_action(&mut self, request: ActionRequest) {
        let mut state = self.0.lock().expect("accessibility state lock poisoned");
        let Some(target) = state.bridge.resolve_target_node(request.target) else {
            return;
        };
        let Some(required_action) = supported_action_for_request(request.action) else {
            return;
        };
        if !state.snapshot.supports_action(target, required_action) {
            return;
        }

        let Some(decoded_request) = decode_action_request(&state.bridge, target, request) else {
            return;
        };
        state.pending_actions.push(decoded_request);
    }
}

impl DeactivationHandler for SharedAccessibilityState {
    fn deactivate_accessibility(&mut self) {}
}

pub struct WindowAccessibilityAdapter {
    adapter: accesskit_winit::Adapter,
    state: Arc<Mutex<AccessibilityAdapterState>>,
}

impl WindowAccessibilityAdapter {
    pub fn new(window: &winit::window::Window, app_name: Option<String>) -> Self {
        let state = Arc::new(Mutex::new(AccessibilityAdapterState::new(app_name)));
        let handler_state = SharedAccessibilityState(state.clone());
        let adapter = accesskit_winit::Adapter::with_direct_handlers(
            window,
            handler_state.clone(),
            handler_state.clone(),
            handler_state,
        );

        Self { adapter, state }
    }

    pub fn process_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) {
        self.adapter.process_event(window, event);
    }

    pub fn update(&mut self, snapshot: AccessibilityTreeSnapshot) {
        {
            let mut state = self
                .state
                .lock()
                .expect("accessibility state lock poisoned");
            state.snapshot = snapshot;
        }

        let state = self.state.clone();
        self.adapter.update_if_active(move || {
            state
                .lock()
                .expect("accessibility state lock poisoned")
                .incremental_tree_update()
        });
    }

    pub fn drain_pending_actions(&mut self) -> Vec<AccessibilityActionRequest> {
        self.state
            .lock()
            .expect("accessibility state lock poisoned")
            .drain_pending_actions()
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
    fn map_text_run_role() {
        assert_eq!(
            map_role(AccessibilityRole::TextRun),
            accesskit::Role::TextRun
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
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Click,
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                ],
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(10.0, 20.0, 100.0, 40.0),
                focused: true,
                children: Vec::new(),
            }],
        };
        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        assert_eq!(update.nodes.len(), 2);
        assert!(update.focus.is_some());
        assert!(update.nodes[1].1.supports_action(Action::Click));
        assert!(update.nodes[1].1.supports_action(Action::Focus));
        assert!(update.nodes[1].1.supports_action(Action::Blur));
    }

    #[test]
    fn convert_empty_snapshot() {
        let snapshot = AccessibilityTreeSnapshot::default();
        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        assert_eq!(update.nodes.len(), 1);
        assert!(update.focus.is_none());
    }

    #[test]
    fn initial_tree_update_includes_tree_metadata() {
        let snapshot = AccessibilityTreeSnapshot::default();
        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_initial_tree_update(&snapshot, Some("Velox Test"));

        assert!(update.tree.is_some());
        let tree = update.tree.expect("initial tree");
        assert_eq!(tree.root, bridge.virtual_root_id());
        assert_eq!(tree.app_name.as_deref(), Some("Velox Test"));
        assert_eq!(tree.toolkit_name.as_deref(), Some("Velox"));
    }

    #[test]
    fn incremental_update_defaults_focus_to_virtual_root() {
        let snapshot = AccessibilityTreeSnapshot::default();
        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_incremental_tree_update(&snapshot);

        assert_eq!(update.focus, bridge.virtual_root_id());
        assert!(update.tree.is_none());
    }

    #[test]
    fn repeated_updates_preserve_accesskit_node_ids() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let first = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Button,
                label: Some("Send".into()),
                value: None,
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Click,
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                ],
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(10.0, 20.0, 100.0, 40.0),
                focused: false,
                children: Vec::new(),
            }],
        };
        let second = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Button,
                label: Some("Send now".into()),
                value: None,
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Click,
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                ],
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(10.0, 20.0, 120.0, 40.0),
                focused: true,
                children: Vec::new(),
            }],
        };

        let mut bridge = AccessibilityBridge::new();
        let first_update = bridge.build_tree_update(&first);
        let second_update = bridge.build_tree_update(&second);

        assert_eq!(first_update.nodes[1].0, second_update.nodes[1].0);
        assert_eq!(first_update.focus, None);
        assert_eq!(second_update.focus, Some(second_update.nodes[1].0));
    }

    #[test]
    fn removed_nodes_are_pruned_from_id_map() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Label,
                label: Some("Hello".into()),
                value: None,
                disabled: false,
                supported_actions: Vec::new(),
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut bridge = AccessibilityBridge::new();
        bridge.build_tree_update(&snapshot);
        assert_eq!(bridge.node_id_map.len(), 1);

        bridge.build_tree_update(&AccessibilityTreeSnapshot::default());
        assert!(bridge.node_id_map.is_empty());
    }

    #[test]
    fn reverse_mapping_resolves_scene_node() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Button,
                label: Some("OK".into()),
                value: None,
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Click,
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                ],
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        let accesskit_id = update.nodes[1].0;

        assert_eq!(bridge.scene_node_for_accesskit(accesskit_id), Some(node_id));
    }

    #[test]
    fn action_handler_queues_only_advertised_actions() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Button,
                label: Some("Open".into()),
                value: None,
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Click,
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                ],
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut adapter_state = AccessibilityAdapterState::new(None);
        let update = adapter_state.bridge.build_tree_update(&snapshot);
        adapter_state.snapshot = snapshot;
        let accesskit_id = update.nodes[1].0;
        let mut shared = SharedAccessibilityState(Arc::new(Mutex::new(adapter_state)));

        shared.do_action(ActionRequest {
            action: Action::Focus,
            target: accesskit_id,
            data: None,
        });
        shared.do_action(ActionRequest {
            action: Action::Blur,
            target: accesskit_id,
            data: None,
        });
        shared.do_action(ActionRequest {
            action: Action::Click,
            target: accesskit_id,
            data: None,
        });
        shared.do_action(ActionRequest {
            action: Action::SetValue,
            target: accesskit_id,
            data: Some(ActionData::Value(String::from("Hello").into_boxed_str())),
        });
        shared.do_action(ActionRequest {
            action: Action::ReplaceSelectedText,
            target: accesskit_id,
            data: Some(ActionData::Value(String::from("World").into_boxed_str())),
        });

        let pending = shared
            .0
            .lock()
            .expect("accessibility state lock poisoned")
            .drain_pending_actions();
        assert_eq!(
            pending,
            vec![
                AccessibilityActionRequest {
                    target: node_id,
                    kind: AccessibilityActionKind::Focus,
                    text: None,
                    selection: None,
                },
                AccessibilityActionRequest {
                    target: node_id,
                    kind: AccessibilityActionKind::Blur,
                    text: None,
                    selection: None,
                },
                AccessibilityActionRequest {
                    target: node_id,
                    kind: AccessibilityActionKind::Click,
                    text: None,
                    selection: None,
                },
            ]
        );
    }

    #[test]
    fn action_handler_ignores_malformed_and_disabled_requests() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::TextInput,
                label: Some("Editor".into()),
                value: Some("Hello".into()),
                disabled: true,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                    velox_scene::AccessibilityActionSupport::SetTextSelection,
                    velox_scene::AccessibilityActionSupport::SetValue,
                    velox_scene::AccessibilityActionSupport::ReplaceSelectedText,
                ],
                text_selection: None,
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 100.0, 40.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut adapter_state = AccessibilityAdapterState::new(None);
        let update = adapter_state.bridge.build_tree_update(&snapshot);
        adapter_state.snapshot = snapshot;
        let accesskit_id = update.nodes[1].0;
        let mut shared = SharedAccessibilityState(Arc::new(Mutex::new(adapter_state)));

        shared.do_action(ActionRequest {
            action: Action::SetValue,
            target: accesskit_id,
            data: None,
        });
        shared.do_action(ActionRequest {
            action: Action::Click,
            target: accesskit_id,
            data: None,
        });

        let pending = shared
            .0
            .lock()
            .expect("accessibility state lock poisoned")
            .drain_pending_actions();
        assert!(pending.is_empty());
    }

    #[test]
    fn text_input_snapshot_adds_text_run_and_selection() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::TextInput,
                label: Some("Editor".into()),
                value: Some("Hello".into()),
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                    velox_scene::AccessibilityActionSupport::SetTextSelection,
                    velox_scene::AccessibilityActionSupport::SetValue,
                    velox_scene::AccessibilityActionSupport::ReplaceSelectedText,
                ],
                text_selection: Some(velox_scene::AccessibilityTextSelection {
                    anchor: 1,
                    focus: 4,
                }),
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 100.0, 40.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        assert_eq!(update.nodes.len(), 3);
        assert!(update.nodes[1].1.text_selection().is_some());
        assert!(update.nodes[1].1.supports_action(Action::SetTextSelection));
        assert!(update.nodes[1].1.supports_action(Action::SetValue));
        assert!(
            update.nodes[1]
                .1
                .supports_action(Action::ReplaceSelectedText)
        );
        assert_eq!(update.nodes[2].1.character_lengths(), &[1, 1, 1, 1, 1]);
    }

    #[test]
    fn label_snapshot_adds_text_run_children() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::Label,
                label: Some("Plain text".into()),
                value: None,
                disabled: false,
                supported_actions: Vec::new(),
                text_selection: None,
                text_runs: vec![velox_scene::AccessibilityTextRun::new(
                    "Plain text",
                    0,
                    Rect::new(5.0, 8.0, 64.0, 16.0),
                )],
                rect: Rect::new(0.0, 0.0, 100.0, 20.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut bridge = AccessibilityBridge::new();
        let update = bridge.build_tree_update(&snapshot);
        assert_eq!(update.nodes.len(), 3);
        assert!(!update.nodes[1].1.supports_action(Action::SetTextSelection));
        assert_eq!(
            update.nodes[2].1.character_lengths(),
            &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
        );
    }

    #[test]
    fn action_handler_decodes_text_selection_against_text_run() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::TextInput,
                label: Some("Editor".into()),
                value: Some("Hello".into()),
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                    velox_scene::AccessibilityActionSupport::SetTextSelection,
                    velox_scene::AccessibilityActionSupport::SetValue,
                    velox_scene::AccessibilityActionSupport::ReplaceSelectedText,
                ],
                text_selection: Some(velox_scene::AccessibilityTextSelection {
                    anchor: 0,
                    focus: 0,
                }),
                text_runs: Vec::new(),
                rect: Rect::new(0.0, 0.0, 100.0, 40.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut adapter_state = AccessibilityAdapterState::new(None);
        let update = adapter_state.bridge.build_tree_update(&snapshot);
        adapter_state.snapshot = snapshot;
        let input_id = update.nodes[1].0;
        let run_id = update.nodes[2].0;
        let mut shared = SharedAccessibilityState(Arc::new(Mutex::new(adapter_state)));

        shared.do_action(ActionRequest {
            action: Action::SetTextSelection,
            target: input_id,
            data: Some(ActionData::SetTextSelection(accesskit::TextSelection {
                anchor: accesskit::TextPosition {
                    node: run_id,
                    character_index: 1,
                },
                focus: accesskit::TextPosition {
                    node: run_id,
                    character_index: 4,
                },
            })),
        });

        let pending = shared
            .0
            .lock()
            .expect("accessibility state lock poisoned")
            .drain_pending_actions();
        assert_eq!(
            pending,
            vec![AccessibilityActionRequest {
                target: node_id,
                kind: AccessibilityActionKind::SetTextSelection,
                text: None,
                selection: Some(velox_scene::AccessibilityTextSelection {
                    anchor: 1,
                    focus: 4,
                }),
            }]
        );
    }

    #[test]
    fn explicit_text_runs_round_trip_across_multiple_runs() {
        let mut map = slotmap::SlotMap::with_key();
        let node_id: NodeId = map.insert(());
        let snapshot = AccessibilityTreeSnapshot {
            roots: vec![AccessibilityTreeNode {
                id: node_id,
                role: AccessibilityRole::TextInput,
                label: Some("Editor".into()),
                value: Some("One\nTwo".into()),
                disabled: false,
                supported_actions: vec![
                    velox_scene::AccessibilityActionSupport::Focus,
                    velox_scene::AccessibilityActionSupport::Blur,
                    velox_scene::AccessibilityActionSupport::SetTextSelection,
                    velox_scene::AccessibilityActionSupport::SetValue,
                    velox_scene::AccessibilityActionSupport::ReplaceSelectedText,
                ],
                text_selection: Some(velox_scene::AccessibilityTextSelection {
                    anchor: 1,
                    focus: 6,
                }),
                text_runs: vec![
                    velox_scene::AccessibilityTextRun::new(
                        "One",
                        0,
                        Rect::new(0.0, 0.0, 30.0, 16.0),
                    ),
                    velox_scene::AccessibilityTextRun::new(
                        "Two",
                        4,
                        Rect::new(0.0, 20.0, 30.0, 16.0),
                    ),
                ],
                rect: Rect::new(0.0, 0.0, 100.0, 60.0),
                focused: false,
                children: Vec::new(),
            }],
        };

        let mut adapter_state = AccessibilityAdapterState::new(None);
        let update = adapter_state.bridge.build_tree_update(&snapshot);
        adapter_state.snapshot = snapshot;
        let input_id = update.nodes[1].0;
        let first_run_id = update.nodes[2].0;
        let second_run_id = update.nodes[3].0;
        assert_eq!(update.nodes.len(), 4);

        let mut shared = SharedAccessibilityState(Arc::new(Mutex::new(adapter_state)));
        shared.do_action(ActionRequest {
            action: Action::SetTextSelection,
            target: input_id,
            data: Some(ActionData::SetTextSelection(accesskit::TextSelection {
                anchor: accesskit::TextPosition {
                    node: first_run_id,
                    character_index: 1,
                },
                focus: accesskit::TextPosition {
                    node: second_run_id,
                    character_index: 2,
                },
            })),
        });

        let pending = shared
            .0
            .lock()
            .expect("accessibility state lock poisoned")
            .drain_pending_actions();
        assert_eq!(
            pending,
            vec![AccessibilityActionRequest {
                target: node_id,
                kind: AccessibilityActionKind::SetTextSelection,
                text: None,
                selection: Some(velox_scene::AccessibilityTextSelection {
                    anchor: 1,
                    focus: 6,
                }),
            }]
        );
    }
}
