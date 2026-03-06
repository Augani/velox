use crate::element::{AnyElement, ElementKey};
use crate::layout_engine::LayoutEngine;
use crate::style::Style;
use std::any::TypeId;
use velox_scene::{NodeId, NodeTree, Point, Rect};

#[derive(Clone)]
pub struct ReconcilerSlot {
    pub node_id: NodeId,
    pub taffy_node: taffy::NodeId,
    pub element_type: TypeId,
    pub key: Option<ElementKey>,
    pub style: Style,
    pub children: Vec<ReconcilerSlot>,
}

#[derive(Debug)]
pub enum Patch {
    UpdateStyle {
        node_id: NodeId,
        taffy_node: taffy::NodeId,
        style: Style,
        layout_changed: bool,
    },
    CreateNode {
        parent: NodeId,
        element_type: TypeId,
        key: Option<ElementKey>,
        style: Style,
    },
    RemoveNode {
        node_id: NodeId,
        taffy_node: taffy::NodeId,
    },
}

pub struct Reconciler {
    slots: Vec<ReconcilerSlot>,
}

impl Reconciler {
    pub fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub fn mount(
        &mut self,
        elements: &mut [AnyElement],
        parent_node: Option<NodeId>,
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) -> Vec<taffy::NodeId> {
        let mut taffy_children = Vec::new();

        for element in elements.iter_mut() {
            let slot = self.mount_element(element, parent_node, tree, engine);
            taffy_children.push(slot.taffy_node);
            self.slots.push(slot);
        }

        taffy_children
    }

    fn mount_element(
        &self,
        element: &mut AnyElement,
        parent_node: Option<NodeId>,
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) -> ReconcilerSlot {
        let node_id = tree.insert(parent_node);
        let style = element.style().clone();

        let child_elements = element.children_mut();
        let mut child_slots = Vec::new();
        let mut child_taffy_nodes = Vec::new();

        for child in child_elements.iter_mut() {
            let child_slot = self.mount_element(child, Some(node_id), tree, engine);
            child_taffy_nodes.push(child_slot.taffy_node);
            child_slots.push(child_slot);
        }

        let taffy_node = if child_taffy_nodes.is_empty() {
            engine.new_leaf(&style).expect("taffy new_leaf")
        } else {
            engine
                .new_with_children(&style, &child_taffy_nodes)
                .expect("taffy new_with_children")
        };

        ReconcilerSlot {
            node_id,
            taffy_node,
            element_type: element.element_type_id(),
            key: element.key,
            style,
            children: child_slots,
        }
    }

    pub fn apply_layout(&self, engine: &LayoutEngine, tree: &mut NodeTree, parent_origin: Point) {
        for slot in &self.slots {
            self.apply_layout_slot(slot, engine, tree, parent_origin);
        }
    }

    fn apply_layout_slot(
        &self,
        slot: &ReconcilerSlot,
        engine: &LayoutEngine,
        tree: &mut NodeTree,
        parent_origin: Point,
    ) {
        let layout = engine.layout(slot.taffy_node).expect("layout exists");
        let rect = Rect::new(
            parent_origin.x + layout.location.x,
            parent_origin.y + layout.location.y,
            layout.size.width,
            layout.size.height,
        );
        tree.set_rect(slot.node_id, rect);

        let child_origin = Point::new(rect.x, rect.y);
        for child in &slot.children {
            self.apply_layout_slot(child, engine, tree, child_origin);
        }
    }

    pub fn slots(&self) -> &[ReconcilerSlot] {
        &self.slots
    }

    pub fn unmount(&self, tree: &mut NodeTree, engine: &mut LayoutEngine) {
        for slot in &self.slots {
            self.unmount_slot(slot, tree, engine);
        }
    }

    fn unmount_slot(&self, slot: &ReconcilerSlot, tree: &mut NodeTree, engine: &mut LayoutEngine) {
        for child in &slot.children {
            self.unmount_slot(child, tree, engine);
        }
        let _ = engine.remove(slot.taffy_node);
        tree.remove(slot.node_id);
    }

    pub fn diff(
        &mut self,
        new_elements: &mut [AnyElement],
        parent_node: Option<NodeId>,
        tree: &mut NodeTree,
        engine: &mut LayoutEngine,
    ) -> Vec<Patch> {
        let mut patches = Vec::new();
        let old_slots = std::mem::take(&mut self.slots);
        let mut new_slots = Vec::new();

        let max_len = old_slots.len().max(new_elements.len());

        for i in 0..max_len {
            match (old_slots.get(i), new_elements.get_mut(i)) {
                (Some(old), Some(new_el)) => {
                    if old.element_type == new_el.element_type_id() {
                        let new_style = new_el.style().clone();
                        let layout_changed = old.style.is_layout_affecting_different(&new_style);
                        if old.style != new_style {
                            patches.push(Patch::UpdateStyle {
                                node_id: old.node_id,
                                taffy_node: old.taffy_node,
                                style: new_style.clone(),
                                layout_changed,
                            });
                        }

                        let mut child_reconciler = Reconciler {
                            slots: old.children.clone(),
                        };
                        let child_patches = child_reconciler.diff(
                            new_el.children_mut(),
                            Some(old.node_id),
                            tree,
                            engine,
                        );
                        patches.extend(child_patches);

                        new_slots.push(ReconcilerSlot {
                            node_id: old.node_id,
                            taffy_node: old.taffy_node,
                            element_type: old.element_type,
                            key: new_el.key,
                            style: new_el.style().clone(),
                            children: child_reconciler.slots,
                        });
                    } else {
                        self.collect_removes(old, &mut patches);
                        let slot = self.mount_element(new_el, parent_node, tree, engine);
                        new_slots.push(slot);
                    }
                }
                (Some(old), None) => {
                    self.collect_removes(old, &mut patches);
                }
                (None, Some(new_el)) => {
                    let slot = self.mount_element(new_el, parent_node, tree, engine);
                    new_slots.push(slot);
                }
                (None, None) => break,
            }
        }

        self.slots = new_slots;
        patches
    }

    fn collect_removes(&self, slot: &ReconcilerSlot, patches: &mut Vec<Patch>) {
        for child in &slot.children {
            self.collect_removes(child, patches);
        }
        patches.push(Patch::RemoveNode {
            node_id: slot.node_id,
            taffy_node: slot.taffy_node,
        });
    }

    pub fn apply_patches(patches: &[Patch], tree: &mut NodeTree, engine: &mut LayoutEngine) {
        for patch in patches {
            match patch {
                Patch::UpdateStyle {
                    node_id,
                    taffy_node,
                    style,
                    layout_changed,
                } => {
                    engine.set_style(*taffy_node, style).ok();
                    if *layout_changed {
                        tree.mark_layout_dirty(*node_id);
                    } else {
                        tree.mark_paint_dirty(*node_id);
                    }
                }
                Patch::RemoveNode {
                    node_id,
                    taffy_node,
                } => {
                    let _ = engine.remove(*taffy_node);
                    tree.remove(*node_id);
                }
                Patch::CreateNode { .. } => {}
            }
        }
    }
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::div;
    use crate::length::px;
    use crate::parent::{IntoAnyElement, ParentElement};
    use crate::styled::Styled;

    #[test]
    fn mount_single_div() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 800.0, 600.0));

        let d = div().w(px(100.0)).h(px(50.0));
        let mut elements = vec![d.into_any_element()];

        let taffy_ids = reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);
        assert_eq!(taffy_ids.len(), 1);
        assert_eq!(reconciler.slots().len(), 1);
        assert!(tree.contains(reconciler.slots()[0].node_id));
    }

    #[test]
    fn mount_nested_div() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 800.0, 600.0));

        let d = div()
            .flex_row()
            .w(px(200.0))
            .h(px(100.0))
            .child(div().w(px(80.0)).h(px(40.0)))
            .child(div().w(px(80.0)).h(px(40.0)));

        let mut elements = vec![d.into_any_element()];
        reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);

        assert_eq!(reconciler.slots().len(), 1);
        assert_eq!(reconciler.slots()[0].children.len(), 2);
    }

    #[test]
    fn mount_and_compute_layout() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 400.0, 300.0));

        let d = div()
            .flex_row()
            .w(px(400.0))
            .h(px(300.0))
            .child(div().w(px(200.0)).h(px(100.0)))
            .child(div().w(px(200.0)).h(px(100.0)));

        let mut elements = vec![d.into_any_element()];
        let taffy_roots = reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);

        let root_taffy = taffy_roots[0];
        engine
            .compute_layout(
                root_taffy,
                taffy::prelude::Size {
                    width: taffy::prelude::AvailableSpace::Definite(400.0),
                    height: taffy::prelude::AvailableSpace::Definite(300.0),
                },
            )
            .unwrap();

        reconciler.apply_layout(&engine, &mut tree, Point::new(0.0, 0.0));

        let child1_id = reconciler.slots()[0].children[0].node_id;
        let child2_id = reconciler.slots()[0].children[1].node_id;

        let r1 = tree.rect(child1_id).unwrap();
        let r2 = tree.rect(child2_id).unwrap();

        assert_eq!(r1.width, 200.0);
        assert_eq!(r2.x, 200.0);
    }

    #[test]
    fn unmount_removes_nodes() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root_node = tree.insert(None);
        let d = div().w(px(100.0));
        let mut elements = vec![d.into_any_element()];
        reconciler.mount(&mut elements, Some(root_node), &mut tree, &mut engine);

        let mounted_id = reconciler.slots()[0].node_id;
        assert!(tree.contains(mounted_id));

        reconciler.unmount(&mut tree, &mut engine);
        assert!(!tree.contains(mounted_id));
    }

    #[test]
    fn diff_updates_style() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 400.0, 300.0));

        let d1 = div()
            .w(px(100.0))
            .h(px(50.0))
            .bg(velox_scene::Color::rgb(255, 0, 0));
        let mut elements1 = vec![d1.into_any_element()];
        reconciler.mount(&mut elements1, Some(root), &mut tree, &mut engine);

        let d2 = div()
            .w(px(100.0))
            .h(px(50.0))
            .bg(velox_scene::Color::rgb(0, 255, 0));
        let mut elements2 = vec![d2.into_any_element()];
        let patches = reconciler.diff(&mut elements2, Some(root), &mut tree, &mut engine);

        assert!(!patches.is_empty());
        assert!(matches!(
            patches[0],
            Patch::UpdateStyle {
                layout_changed: false,
                ..
            }
        ));
    }

    #[test]
    fn diff_detects_layout_change() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root = tree.insert(None);
        let d1 = div().w(px(100.0));
        let mut elements1 = vec![d1.into_any_element()];
        reconciler.mount(&mut elements1, Some(root), &mut tree, &mut engine);

        let d2 = div().w(px(200.0));
        let mut elements2 = vec![d2.into_any_element()];
        let patches = reconciler.diff(&mut elements2, Some(root), &mut tree, &mut engine);

        assert!(matches!(
            patches[0],
            Patch::UpdateStyle {
                layout_changed: true,
                ..
            }
        ));
    }

    #[test]
    fn diff_removes_extra_elements() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root = tree.insert(None);
        let mut els = vec![div().into_any_element(), div().into_any_element()];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine);
        assert_eq!(reconciler.slots().len(), 2);

        let mut els2 = vec![div().into_any_element()];
        let patches = reconciler.diff(&mut els2, Some(root), &mut tree, &mut engine);

        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::RemoveNode { .. })));
        assert_eq!(reconciler.slots().len(), 1);
    }

    #[test]
    fn diff_adds_new_elements() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();

        let root = tree.insert(None);
        let mut els = vec![div().into_any_element()];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine);

        let mut els2 = vec![
            div().into_any_element(),
            div().w(px(50.0)).into_any_element(),
        ];
        let _patches = reconciler.diff(&mut els2, Some(root), &mut tree, &mut engine);
        assert_eq!(reconciler.slots().len(), 2);
    }
}
