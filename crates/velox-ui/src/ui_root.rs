use crate::coordinator::UiCoordinator;
use crate::element::{AccessibilityAction, AnyElement, PaintContext};
use crate::layout_engine::LayoutEngine;
use crate::reconciler::{Reconciler, ReconcilerSlot};
use velox_scene::{
    AccessibilityRole, AccessibilityTreeNode, AccessibilityTreeSnapshot, NodeTree, Point, Rect,
};

pub struct UiRoot {
    reconciler: Reconciler,
    engine: LayoutEngine,
    tree: NodeTree,
    coordinator: UiCoordinator,
    root_elements: Vec<AnyElement>,
    root_node: velox_scene::NodeId,
    taffy_roots: Vec<taffy::NodeId>,
    needs_layout: bool,
}

impl UiRoot {
    pub fn new() -> Self {
        let mut tree = NodeTree::new();
        let root_node = tree.insert(None);
        Self {
            reconciler: Reconciler::new(),
            engine: LayoutEngine::new(),
            tree,
            coordinator: UiCoordinator::new(),
            root_elements: Vec::new(),
            root_node,
            taffy_roots: Vec::new(),
            needs_layout: true,
        }
    }

    pub fn set_root(
        &mut self,
        mut elements: Vec<AnyElement>,
        font_system: &mut velox_text::FontSystem,
    ) {
        let old_focus_path = self
            .coordinator
            .focused_node()
            .and_then(|node| Self::find_slot_path(self.reconciler.slots(), node));
        let old_hover_path = self
            .coordinator
            .hovered_node()
            .and_then(|node| Self::find_slot_path(self.reconciler.slots(), node));

        if self.reconciler.slots().is_empty() {
            self.root_elements = elements;
            self.taffy_roots = self.reconciler.mount(
                &mut self.root_elements,
                Some(self.root_node),
                &mut self.tree,
                &mut self.engine,
                font_system,
            );
        } else {
            let patches = self.reconciler.diff(
                &mut elements,
                Some(self.root_node),
                &mut self.tree,
                &mut self.engine,
                font_system,
            );
            Reconciler::apply_patches(&patches, &mut self.tree, &mut self.engine);
            self.root_elements = elements;
            self.taffy_roots = self
                .reconciler
                .slots()
                .iter()
                .map(|slot| slot.taffy_node)
                .collect();
        }
        self.needs_layout = true;

        let slots_mut = self.reconciler.slots_mut();
        self.coordinator.build_dispatch_from_slots(slots_mut);

        if let Some(path) = old_focus_path {
            if let Some(new_node) = Self::node_at_path(self.reconciler.slots(), &path) {
                self.coordinator.focus_manager_mut().request_focus(new_node);
            } else {
                self.coordinator.focus_manager_mut().clear_focus();
            }
        }
        if let Some(path) = old_hover_path {
            if let Some(new_node) = Self::node_at_path(self.reconciler.slots(), &path) {
                self.coordinator
                    .hover_manager_mut()
                    .set_hovered(Some(new_node));
            } else {
                self.coordinator.hover_manager_mut().set_hovered(None);
            }
        }
    }

    fn find_slot_path(slots: &[ReconcilerSlot], target: velox_scene::NodeId) -> Option<Vec<usize>> {
        for (i, slot) in slots.iter().enumerate() {
            if slot.node_id == target {
                return Some(vec![i]);
            }
            if let Some(mut path) = Self::find_slot_path(&slot.children, target) {
                path.insert(0, i);
                return Some(path);
            }
        }
        None
    }

    fn node_at_path(slots: &[ReconcilerSlot], path: &[usize]) -> Option<velox_scene::NodeId> {
        let idx = *path.first()?;
        let slot = slots.get(idx)?;
        if path.len() == 1 {
            Some(slot.node_id)
        } else {
            Self::node_at_path(&slot.children, &path[1..])
        }
    }

    fn element_at_path_mut<'a>(
        elements: &'a mut [AnyElement],
        path: &[usize],
    ) -> Option<&'a mut AnyElement> {
        let idx = *path.first()?;
        let element = elements.get_mut(idx)?;
        if path.len() == 1 {
            Some(element)
        } else {
            Self::element_at_path_mut(element.children_mut(), &path[1..])
        }
    }

    pub fn layout(&mut self, width: f32, height: f32) {
        self.tree
            .set_rect(self.root_node, Rect::new(0.0, 0.0, width, height));

        for &taffy_root in &self.taffy_roots {
            self.engine
                .compute_layout(
                    taffy_root,
                    taffy::prelude::Size {
                        width: taffy::prelude::AvailableSpace::Definite(width),
                        height: taffy::prelude::AvailableSpace::Definite(height),
                    },
                )
                .ok();
        }

        self.reconciler
            .apply_layout(&self.engine, &mut self.tree, Point::new(0.0, 0.0));
        self.needs_layout = false;
    }

    pub fn paint(&mut self, cx: &mut PaintContext) {
        let slots = self.reconciler.slots().to_vec();
        for (element, slot) in self.root_elements.iter_mut().zip(slots.iter()) {
            Self::paint_element(&self.engine, element, slot, Point::new(0.0, 0.0), cx);
        }
    }

    fn paint_element(
        engine: &LayoutEngine,
        element: &mut AnyElement,
        slot: &ReconcilerSlot,
        parent_origin: Point,
        cx: &mut PaintContext,
    ) {
        let Ok(layout) = engine.layout(slot.taffy_node) else {
            return;
        };
        let rect = Rect::new(
            parent_origin.x + layout.location.x,
            parent_origin.y + layout.location.y,
            layout.size.width,
            layout.size.height,
        );

        element.paint(rect, cx);

        let child_origin = Point::new(rect.x, rect.y);
        let (scroll_ox, scroll_oy) = cx.scroll_offset();
        let adjusted_origin = Point::new(child_origin.x - scroll_ox, child_origin.y - scroll_oy);

        let children = element.children_mut();
        for (child_el, child_slot) in children.iter_mut().zip(slot.children.iter()) {
            Self::paint_element(engine, child_el, child_slot, adjusted_origin, cx);
        }

        element.paint_after_children(rect, cx);
    }

    pub fn coordinator(&self) -> &UiCoordinator {
        &self.coordinator
    }

    pub fn coordinator_mut(&mut self) -> &mut UiCoordinator {
        &mut self.coordinator
    }

    pub fn build_accessibility_tree(&mut self) -> AccessibilityTreeSnapshot {
        let focused = self.coordinator.focused_node();
        let mut roots = Vec::new();

        for (element, slot) in self
            .root_elements
            .iter_mut()
            .zip(self.reconciler.slots().iter())
        {
            let result = Self::build_accessibility_subtree(&self.tree, element, slot, focused);
            if result.nodes.is_empty() {
                if let Some(node) = synthesize_text_only_accessibility_root(
                    &self.tree,
                    slot.node_id,
                    focused,
                    result,
                ) {
                    roots.push(node);
                }
            } else {
                roots.extend(result.nodes);
            }
        }

        AccessibilityTreeSnapshot { roots }
    }

    pub fn request_accessibility_focus(&mut self, node: velox_scene::NodeId) -> bool {
        self.coordinator
            .handle_accessibility_focus(node)
            .needs_redraw
    }

    pub fn clear_accessibility_focus(&mut self, node: velox_scene::NodeId) -> bool {
        self.coordinator
            .handle_accessibility_blur(node)
            .needs_redraw
    }

    pub fn set_accessibility_value(&mut self, node: velox_scene::NodeId, value: String) -> bool {
        self.handle_accessibility_action(node, AccessibilityAction::SetValue(value))
    }

    pub fn replace_accessibility_selected_text(
        &mut self,
        node: velox_scene::NodeId,
        value: String,
    ) -> bool {
        self.handle_accessibility_action(node, AccessibilityAction::ReplaceSelectedText(value))
    }

    pub fn set_accessibility_text_selection(
        &mut self,
        node: velox_scene::NodeId,
        selection: velox_scene::AccessibilityTextSelection,
    ) -> bool {
        self.handle_accessibility_action(node, AccessibilityAction::SetTextSelection(selection))
    }

    pub fn activate_accessibility(&mut self, node: velox_scene::NodeId) -> bool {
        let center = self.node_rect(node).map_or(Point::new(0.0, 0.0), |rect| {
            Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
        });
        self.coordinator
            .handle_accessibility_click(node, center)
            .needs_redraw
    }

    fn handle_accessibility_action(
        &mut self,
        node: velox_scene::NodeId,
        action: AccessibilityAction,
    ) -> bool {
        let Some(path) = Self::find_slot_path(self.reconciler.slots(), node) else {
            return false;
        };
        let Some(element) = Self::element_at_path_mut(&mut self.root_elements, &path) else {
            return false;
        };
        let handled = element.handle_accessibility_action(&action);
        if handled {
            self.needs_layout = true;
        }
        handled
    }

    pub fn hit_test(&self, point: Point) -> Option<velox_scene::NodeId> {
        self.tree.hit_test(point)
    }

    pub fn node_rect(&self, node: velox_scene::NodeId) -> Option<Rect> {
        self.tree.rect(node)
    }

    pub fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    pub fn mark_needs_layout(&mut self) {
        self.needs_layout = true;
    }

    fn build_accessibility_subtree(
        tree: &NodeTree,
        element: &mut AnyElement,
        slot: &ReconcilerSlot,
        focused: Option<velox_scene::NodeId>,
    ) -> AccessibilityBuildResult {
        let mut child_nodes = Vec::new();
        let mut child_text_fragments = Vec::new();
        let mut child_text_runs = Vec::new();
        {
            let children = element.children_mut();
            for (child_el, child_slot) in children.iter_mut().zip(slot.children.iter()) {
                let result = Self::build_accessibility_subtree(tree, child_el, child_slot, focused);
                child_nodes.extend(result.nodes);
                if !result.text_content.is_empty() {
                    child_text_fragments.push(result.text_content);
                }
                child_text_runs.extend(result.text_runs);
            }
        }

        let mut info = element.accessibility();
        let child_text = join_text_fragments(child_text_fragments);
        let node_rect = tree.rect(slot.node_id).unwrap_or_else(Rect::zero);
        let local_text_runs = std::mem::take(&mut info.text_runs);
        let absolute_text_runs = offset_text_runs(local_text_runs, node_rect);

        if let Some(mut node) = info.node.take() {
            if node.label.is_none() && !child_text.is_empty() {
                node.label = Some(child_text.clone());
            }

            let text_content = info
                .text_content
                .clone()
                .or_else(|| node.label.clone())
                .unwrap_or_else(|| child_text.clone());
            let text_runs = if absolute_text_runs.is_empty() {
                child_text_runs
            } else {
                absolute_text_runs
            };

            AccessibilityBuildResult {
                text_content,
                text_runs: Vec::new(),
                nodes: vec![AccessibilityTreeNode {
                    id: slot.node_id,
                    role: node.role,
                    label: node.label,
                    value: node.value,
                    disabled: node.disabled,
                    supported_actions: node.supported_actions,
                    text_selection: node.text_selection,
                    text_runs,
                    rect: node_rect,
                    focused: focused == Some(slot.node_id),
                    children: child_nodes,
                }],
            }
        } else {
            let mut text_fragments = Vec::new();
            if let Some(text) = info.text_content
                && !text.trim().is_empty() {
                    text_fragments.push(text);
                }
            if !child_text.is_empty() {
                text_fragments.push(child_text);
            }

            let mut text_runs = absolute_text_runs;
            text_runs.extend(child_text_runs);

            AccessibilityBuildResult {
                nodes: child_nodes,
                text_content: join_text_fragments(text_fragments),
                text_runs,
            }
        }
    }
}

struct AccessibilityBuildResult {
    nodes: Vec<AccessibilityTreeNode>,
    text_content: String,
    text_runs: Vec<velox_scene::AccessibilityTextRun>,
}

fn join_text_fragments(fragments: Vec<String>) -> String {
    fragments
        .into_iter()
        .filter_map(|fragment| {
            let trimmed = fragment.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn offset_text_runs(
    runs: Vec<velox_scene::AccessibilityTextRun>,
    node_rect: Rect,
) -> Vec<velox_scene::AccessibilityTextRun> {
    runs.into_iter()
        .map(|run| velox_scene::AccessibilityTextRun {
            text: run.text,
            byte_start: run.byte_start,
            rect: Rect::new(
                node_rect.x + run.rect.x,
                node_rect.y + run.rect.y,
                run.rect.width,
                run.rect.height,
            ),
        })
        .collect()
}

fn synthesize_text_only_accessibility_root(
    tree: &NodeTree,
    node_id: velox_scene::NodeId,
    focused: Option<velox_scene::NodeId>,
    result: AccessibilityBuildResult,
) -> Option<AccessibilityTreeNode> {
    let label = result.text_content.trim().to_string();
    if label.is_empty() && result.text_runs.is_empty() {
        return None;
    }

    Some(AccessibilityTreeNode {
        id: node_id,
        role: AccessibilityRole::Label,
        label: (!label.is_empty()).then_some(label),
        value: None,
        disabled: false,
        supported_actions: Vec::new(),
        text_selection: None,
        text_runs: result.text_runs,
        rect: tree.rect(node_id).unwrap_or_else(Rect::zero),
        focused: focused == Some(node_id),
        children: Vec::new(),
    })
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::accessibility::AccessibleElement;
    use crate::element::IntoElement;
    use crate::elements::div;
    use crate::elements::input;
    use crate::elements::text;
    use crate::interactive::InteractiveElement;
    use crate::length::px;
    use crate::parent::{IntoAnyElement, ParentElement};
    use crate::styled::Styled;
    use crate::InputHandle;
    use velox_scene::{AccessibilityRole, Color};

    fn paint_root(root: &mut UiRoot) {
        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut fs,
            glyph_rasterizer: &mut gr,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };

        root.paint(&mut cx);
    }

    #[test]
    fn ui_root_mount_and_layout() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        let d = div().w(px(200.0)).h(px(100.0)).bg(Color::rgb(255, 0, 0));
        root.set_root(vec![d.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        assert!(!root.needs_layout());
    }

    #[test]
    fn ui_root_paint_emits_commands() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        let d = div().w(px(100.0)).h(px(50.0)).bg(Color::rgb(255, 0, 0));
        root.set_root(vec![d.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut fs,
            glyph_rasterizer: &mut gr,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };

        root.paint(&mut cx);

        assert!(!commands.commands().is_empty());
    }

    #[test]
    fn ui_root_nested_layout_and_paint() {
        use crate::parent::ParentElement;

        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        let d = div()
            .flex_row()
            .w(px(400.0))
            .h(px(200.0))
            .bg(Color::rgb(240, 240, 240))
            .child(div().w(px(200.0)).h(px(100.0)).bg(Color::rgb(255, 0, 0)))
            .child(div().w(px(200.0)).h(px(100.0)).bg(Color::rgb(0, 0, 255)));
        root.set_root(vec![d.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut fs,
            glyph_rasterizer: &mut gr,
            hovered_node: None,
            active_node: None,
            focused_node: None,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            scale_factor: 1.0,
        };

        root.paint(&mut cx);

        let fill_count = commands
            .commands()
            .iter()
            .filter(|c| matches!(c, velox_scene::PaintCommand::FillRect { .. }))
            .count();
        assert_eq!(fill_count, 3);
    }

    #[test]
    fn set_root_reuses_keyed_nodes() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        let first = div()
            .child(div().w(px(100.0)).h(px(40.0)).key(7))
            .into_any_element();
        root.set_root(vec![first], &mut fs);
        let original_node = root.reconciler.slots()[0].children[0].node_id;
        let original_taffy = root.reconciler.slots()[0].children[0].taffy_node;

        let second = div()
            .bg(Color::rgb(10, 10, 10))
            .child(
                div()
                    .w(px(120.0))
                    .h(px(40.0))
                    .bg(Color::rgb(255, 0, 0))
                    .key(7),
            )
            .into_any_element();
        root.set_root(vec![second], &mut fs);

        assert_eq!(
            root.reconciler.slots()[0].children[0].node_id,
            original_node
        );
        assert_eq!(
            root.reconciler.slots()[0].children[0].taffy_node,
            original_taffy
        );
    }

    #[test]
    fn set_root_updates_intrinsic_text_layout() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        root.set_root(vec![text("Hi").into_any_element()], &mut fs);
        root.layout(800.0, 600.0);
        let short_width = root
            .engine
            .layout(root.reconciler.slots()[0].taffy_node)
            .expect("initial layout should exist")
            .size
            .width;

        root.set_root(
            vec![text("Hello, Velox layout").into_any_element()],
            &mut fs,
        );
        root.layout(800.0, 600.0);
        let long_width = root
            .engine
            .layout(root.reconciler.slots()[0].taffy_node)
            .expect("updated layout should exist")
            .size
            .width;

        assert!(long_width > short_width);
    }

    #[test]
    fn build_accessibility_tree_uses_semantics_and_descendant_text() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        let button = div()
            .accessibility_role(AccessibilityRole::Button)
            .focusable()
            .child(text("Save"));
        root.set_root(vec![button.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let snapshot = root.build_accessibility_tree();
        assert_eq!(snapshot.node_count(), 1);
        assert_eq!(snapshot.roots[0].role, AccessibilityRole::Button);
        assert_eq!(snapshot.roots[0].label.as_deref(), Some("Save"));
    }

    #[test]
    fn standalone_text_synthesizes_accessible_label_root() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();

        root.set_root(vec![text("Plain text").into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let snapshot = root.build_accessibility_tree();
        assert_eq!(snapshot.node_count(), 1);
        assert_eq!(snapshot.roots[0].role, AccessibilityRole::Label);
        assert_eq!(snapshot.roots[0].label.as_deref(), Some("Plain text"));
        assert_eq!(snapshot.roots[0].text_runs.len(), 1);
        assert_eq!(snapshot.roots[0].text_runs[0].text, "Plain text");
    }

    #[test]
    fn accessibility_actions_focus_and_click_ui_nodes() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();
        let clicks = Rc::new(Cell::new(0usize));
        let click_counter = clicks.clone();

        let button = div()
            .accessibility_role(AccessibilityRole::Button)
            .focusable()
            .on_click(move |_| click_counter.set(click_counter.get() + 1))
            .child(text("Press"));
        root.set_root(vec![button.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let node = root.reconciler.slots()[0].node_id;
        assert!(root.request_accessibility_focus(node));
        assert_eq!(root.coordinator().focused_node(), Some(node));

        assert!(root.activate_accessibility(node));
        assert_eq!(clicks.get(), 1);

        assert!(root.clear_accessibility_focus(node));
        assert_eq!(root.coordinator().focused_node(), None);
    }

    #[test]
    fn accessibility_actions_can_set_and_replace_input_value() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();
        let handle = InputHandle::new();

        let field = input().handle(handle.clone()).focusable();
        root.set_root(vec![field.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let node = root.reconciler.slots()[0].node_id;
        assert!(root.set_accessibility_value(node, String::from("Hello")));
        paint_root(&mut root);
        assert_eq!(handle.text(), "Hello");

        assert!(root.set_accessibility_text_selection(
            node,
            velox_scene::AccessibilityTextSelection {
                anchor: 1,
                focus: 4,
            },
        ));
        paint_root(&mut root);
        assert!(root.replace_accessibility_selected_text(node, String::from("World")));
        paint_root(&mut root);
        assert_eq!(handle.text(), "HWorldo");

        let snapshot = root.build_accessibility_tree();
        assert_eq!(snapshot.roots[0].value.as_deref(), Some("HWorldo"));
        assert_eq!(
            snapshot.roots[0].text_selection,
            Some(velox_scene::AccessibilityTextSelection {
                anchor: 6,
                focus: 6,
            })
        );
    }

    #[test]
    fn multiline_input_accessibility_uses_multiple_text_runs() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();
        let handle = InputHandle::new();

        let field = input()
            .handle(handle)
            .initial_value("One\nTwo")
            .multiline()
            .focusable();
        root.set_root(vec![field.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let snapshot = root.build_accessibility_tree();
        let runs = &snapshot.roots[0].text_runs;
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "One");
        assert_eq!(runs[1].text, "Two");
        assert!(runs[1].rect.y > runs[0].rect.y);
    }

    #[test]
    fn crlf_input_accessibility_preserves_second_line_offset() {
        let mut root = UiRoot::new();
        let mut fs = velox_text::FontSystem::new();
        let handle = InputHandle::new();

        let field = input()
            .handle(handle)
            .initial_value("One\r\nTwo")
            .multiline()
            .focusable();
        root.set_root(vec![field.into_any_element()], &mut fs);
        root.layout(800.0, 600.0);

        let snapshot = root.build_accessibility_tree();
        let runs = &snapshot.roots[0].text_runs;
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[1].text, "Two");
        assert_eq!(runs[1].byte_start, 5);
    }
}
