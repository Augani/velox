use std::collections::HashMap;

use crate::element::{AnyElement, ElementKey, LayoutContext};
use crate::interactive::EventHandlers;
use crate::layout_engine::LayoutEngine;
use crate::style::Style;
use std::any::TypeId;
use velox_scene::{NodeId, NodeTree, Point, Rect};

pub struct ReconcilerSlot {
    pub node_id: NodeId,
    pub taffy_node: taffy::NodeId,
    pub element_type: TypeId,
    pub key: Option<ElementKey>,
    pub style: Style,
    pub taffy_style: taffy::Style,
    pub handlers: Option<EventHandlers>,
    pub children: Vec<ReconcilerSlot>,
}

impl Clone for ReconcilerSlot {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id,
            taffy_node: self.taffy_node,
            element_type: self.element_type,
            key: self.key,
            style: self.style.clone(),
            taffy_style: self.taffy_style.clone(),
            handlers: self.handlers.clone(),
            children: self.children.clone(),
        }
    }
}

#[derive(Debug)]
pub enum Patch {
    UpdateNode {
        node_id: NodeId,
        taffy_node: taffy::NodeId,
        taffy_style: taffy::Style,
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
        font_system: &mut velox_text::FontSystem,
    ) -> Vec<taffy::NodeId> {
        let mut taffy_children = Vec::new();

        for element in elements.iter_mut() {
            let slot = self.mount_element(element, parent_node, tree, engine, font_system);
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
        font_system: &mut velox_text::FontSystem,
    ) -> ReconcilerSlot {
        let node_id = tree.insert(parent_node);
        let handlers = element.take_handlers();

        let (child_slots, child_taffy_nodes) = {
            let child_elements = element.children_mut();
            let mut slots = Vec::new();
            let mut taffy_nodes = Vec::new();

            for child in child_elements.iter_mut() {
                let child_slot =
                    self.mount_element(child, Some(node_id), tree, engine, font_system);
                taffy_nodes.push(child_slot.taffy_node);
                slots.push(child_slot);
            }

            (slots, taffy_nodes)
        };

        self.finish_mount(
            element,
            node_id,
            handlers,
            child_slots,
            child_taffy_nodes,
            engine,
            font_system,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_mount(
        &self,
        element: &mut AnyElement,
        node_id: NodeId,
        handlers: EventHandlers,
        child_slots: Vec<ReconcilerSlot>,
        child_taffy_nodes: Vec<taffy::NodeId>,
        engine: &mut LayoutEngine,
        font_system: &mut velox_text::FontSystem,
    ) -> ReconcilerSlot {
        let layout_req = {
            let mut layout_cx = LayoutContext {
                taffy: &mut engine.taffy,
                font_system,
            };
            element.layout(&mut layout_cx)
        };
        let style = element.style().clone();

        let taffy_style = layout_req.taffy_style;

        let taffy_node = if child_taffy_nodes.is_empty() {
            engine
                .taffy
                .new_leaf(taffy_style.clone())
                .expect("taffy new_leaf")
        } else {
            engine
                .taffy
                .new_with_children(taffy_style.clone(), &child_taffy_nodes)
                .expect("taffy new_with_children")
        };

        ReconcilerSlot {
            node_id,
            taffy_node,
            element_type: element.element_type_id(),
            key: element.key,
            style,
            taffy_style,
            handlers: Some(handlers),
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

    pub fn slots_mut(&mut self) -> &mut [ReconcilerSlot] {
        &mut self.slots
    }

    pub fn unmount(&mut self, tree: &mut NodeTree, engine: &mut LayoutEngine) {
        for slot in &self.slots {
            self.unmount_slot(slot, tree, engine);
        }
        self.slots.clear();
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
        font_system: &mut velox_text::FontSystem,
    ) -> Vec<Patch> {
        let mut patches = Vec::new();
        let old_slots = std::mem::take(&mut self.slots);
        let mut new_slots = Vec::new();

        let mut old_keyed: HashMap<ElementKey, usize> = HashMap::new();
        let mut old_used = vec![false; old_slots.len()];
        for (i, slot) in old_slots.iter().enumerate() {
            if let Some(key) = slot.key {
                old_keyed.insert(key, i);
            }
        }

        let mut old_unkeyed_cursor = 0;

        for new_el in new_elements.iter_mut() {
            let matched = if let Some(key) = new_el.key {
                old_keyed.get(&key).copied().filter(|&i| !old_used[i])
            } else {
                loop {
                    if old_unkeyed_cursor >= old_slots.len() {
                        break None;
                    }
                    if !old_used[old_unkeyed_cursor] && old_slots[old_unkeyed_cursor].key.is_none()
                    {
                        break Some(old_unkeyed_cursor);
                    }
                    old_unkeyed_cursor += 1;
                }
            };

            if let Some(idx) = matched {
                old_used[idx] = true;
                let old = &old_slots[idx];

                if old.element_type == new_el.element_type_id() {
                    let layout_req = {
                        let mut layout_cx = LayoutContext {
                            taffy: &mut engine.taffy,
                            font_system,
                        };
                        new_el.layout(&mut layout_cx)
                    };
                    let new_style = new_el.style().clone();
                    let new_taffy_style = layout_req.taffy_style;
                    let layout_changed = old.taffy_style != new_taffy_style;
                    if old.style != new_style || layout_changed {
                        patches.push(Patch::UpdateNode {
                            node_id: old.node_id,
                            taffy_node: old.taffy_node,
                            taffy_style: new_taffy_style.clone(),
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
                        font_system,
                    );
                    patches.extend(child_patches);
                    let new_child_taffy_nodes: Vec<_> = child_reconciler
                        .slots
                        .iter()
                        .map(|slot| slot.taffy_node)
                        .collect();
                    let old_child_taffy_nodes: Vec<_> =
                        old.children.iter().map(|slot| slot.taffy_node).collect();
                    let children_changed = old_child_taffy_nodes != new_child_taffy_nodes;
                    if children_changed {
                        engine
                            .set_children(old.taffy_node, &new_child_taffy_nodes)
                            .ok();
                    }
                    let new_handlers = new_el.take_handlers();
                    new_slots.push(ReconcilerSlot {
                        node_id: old.node_id,
                        taffy_node: old.taffy_node,
                        element_type: old.element_type,
                        key: new_el.key,
                        style: new_el.style().clone(),
                        taffy_style: new_taffy_style,
                        handlers: Some(new_handlers),
                        children: child_reconciler.slots,
                    });
                } else {
                    self.collect_removes(old, &mut patches);
                    let slot = self.mount_element(new_el, parent_node, tree, engine, font_system);
                    new_slots.push(slot);
                }
            } else {
                let slot = self.mount_element(new_el, parent_node, tree, engine, font_system);
                new_slots.push(slot);
            }
        }

        for (i, slot) in old_slots.iter().enumerate() {
            if !old_used[i] {
                self.collect_removes(slot, &mut patches);
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
                Patch::UpdateNode {
                    node_id,
                    taffy_node,
                    taffy_style,
                    layout_changed,
                } => {
                    engine
                        .set_taffy_style(*taffy_node, taffy_style.clone())
                        .ok();
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
        let mut fs = velox_text::FontSystem::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 800.0, 600.0));

        let d = div().w(px(100.0)).h(px(50.0));
        let mut elements = vec![d.into_any_element()];

        let taffy_ids = reconciler.mount(
            &mut elements,
            Some(root_node),
            &mut tree,
            &mut engine,
            &mut fs,
        );
        assert_eq!(taffy_ids.len(), 1);
        assert_eq!(reconciler.slots().len(), 1);
        assert!(tree.contains(reconciler.slots()[0].node_id));
    }

    #[test]
    fn mount_nested_div() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();
        let mut fs = velox_text::FontSystem::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 800.0, 600.0));

        let d = div()
            .flex_row()
            .w(px(200.0))
            .h(px(100.0))
            .child(div().w(px(80.0)).h(px(40.0)))
            .child(div().w(px(80.0)).h(px(40.0)));

        let mut elements = vec![d.into_any_element()];
        reconciler.mount(
            &mut elements,
            Some(root_node),
            &mut tree,
            &mut engine,
            &mut fs,
        );

        assert_eq!(reconciler.slots().len(), 1);
        assert_eq!(reconciler.slots()[0].children.len(), 2);
    }

    #[test]
    fn mount_and_compute_layout() {
        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();
        let mut fs = velox_text::FontSystem::new();

        let root_node = tree.insert(None);
        tree.set_rect(root_node, Rect::new(0.0, 0.0, 400.0, 300.0));

        let d = div()
            .flex_row()
            .w(px(400.0))
            .h(px(300.0))
            .child(div().w(px(200.0)).h(px(100.0)))
            .child(div().w(px(200.0)).h(px(100.0)));

        let mut elements = vec![d.into_any_element()];
        let taffy_roots = reconciler.mount(
            &mut elements,
            Some(root_node),
            &mut tree,
            &mut engine,
            &mut fs,
        );

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
        let mut fs = velox_text::FontSystem::new();

        let root_node = tree.insert(None);
        let d = div().w(px(100.0));
        let mut elements = vec![d.into_any_element()];
        reconciler.mount(
            &mut elements,
            Some(root_node),
            &mut tree,
            &mut engine,
            &mut fs,
        );

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
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 400.0, 300.0));

        let d1 = div()
            .w(px(100.0))
            .h(px(50.0))
            .bg(velox_scene::Color::rgb(255, 0, 0));
        let mut elements1 = vec![d1.into_any_element()];
        reconciler.mount(&mut elements1, Some(root), &mut tree, &mut engine, &mut fs);

        let d2 = div()
            .w(px(100.0))
            .h(px(50.0))
            .bg(velox_scene::Color::rgb(0, 255, 0));
        let mut elements2 = vec![d2.into_any_element()];
        let patches = reconciler.diff(&mut elements2, Some(root), &mut tree, &mut engine, &mut fs);

        assert!(!patches.is_empty());
        assert!(matches!(
            patches[0],
            Patch::UpdateNode {
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
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);
        let d1 = div().w(px(100.0));
        let mut elements1 = vec![d1.into_any_element()];
        reconciler.mount(&mut elements1, Some(root), &mut tree, &mut engine, &mut fs);

        let d2 = div().w(px(200.0));
        let mut elements2 = vec![d2.into_any_element()];
        let patches = reconciler.diff(&mut elements2, Some(root), &mut tree, &mut engine, &mut fs);

        assert!(matches!(
            patches[0],
            Patch::UpdateNode {
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
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);
        let mut els = vec![div().into_any_element(), div().into_any_element()];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine, &mut fs);
        assert_eq!(reconciler.slots().len(), 2);

        let mut els2 = vec![div().into_any_element()];
        let patches = reconciler.diff(&mut els2, Some(root), &mut tree, &mut engine, &mut fs);

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
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);
        let mut els = vec![div().into_any_element()];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine, &mut fs);

        let mut els2 = vec![
            div().into_any_element(),
            div().w(px(50.0)).into_any_element(),
        ];
        let _patches = reconciler.diff(&mut els2, Some(root), &mut tree, &mut engine, &mut fs);
        assert_eq!(reconciler.slots().len(), 2);
    }

    #[test]
    fn keyed_reorder_preserves_node_ids() {
        use crate::element::IntoElement;

        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);

        let mut els = vec![
            div().w(px(10.0)).key(1).into_any_element(),
            div().w(px(20.0)).key(2).into_any_element(),
            div().w(px(30.0)).key(3).into_any_element(),
        ];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine, &mut fs);

        let old_node_1 = reconciler.slots()[0].node_id;
        let old_node_2 = reconciler.slots()[1].node_id;
        let old_node_3 = reconciler.slots()[2].node_id;

        let mut reordered = vec![
            div().w(px(30.0)).key(3).into_any_element(),
            div().w(px(10.0)).key(1).into_any_element(),
            div().w(px(20.0)).key(2).into_any_element(),
        ];
        reconciler.diff(&mut reordered, Some(root), &mut tree, &mut engine, &mut fs);

        assert_eq!(reconciler.slots()[0].node_id, old_node_3);
        assert_eq!(reconciler.slots()[1].node_id, old_node_1);
        assert_eq!(reconciler.slots()[2].node_id, old_node_2);
    }

    #[test]
    fn keyed_insert_and_remove() {
        use crate::element::IntoElement;

        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);

        let mut els = vec![
            div().key(1).into_any_element(),
            div().key(2).into_any_element(),
        ];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine, &mut fs);
        let node_2 = reconciler.slots()[1].node_id;

        let mut new_els = vec![
            div().key(2).into_any_element(),
            div().key(3).into_any_element(),
        ];
        let patches = reconciler.diff(&mut new_els, Some(root), &mut tree, &mut engine, &mut fs);

        assert_eq!(reconciler.slots()[0].node_id, node_2);
        assert!(patches
            .iter()
            .any(|p| matches!(p, Patch::RemoveNode { .. })));
    }

    #[test]
    fn mixed_keyed_unkeyed() {
        use crate::element::IntoElement;

        let mut tree = NodeTree::new();
        let mut engine = LayoutEngine::new();
        let mut reconciler = Reconciler::new();
        let mut fs = velox_text::FontSystem::new();

        let root = tree.insert(None);

        let mut els = vec![div().key(1).into_any_element(), div().into_any_element()];
        reconciler.mount(&mut els, Some(root), &mut tree, &mut engine, &mut fs);
        let keyed_node = reconciler.slots()[0].node_id;
        let unkeyed_node = reconciler.slots()[1].node_id;

        let mut new_els = vec![div().into_any_element(), div().key(1).into_any_element()];
        reconciler.diff(&mut new_els, Some(root), &mut tree, &mut engine, &mut fs);

        assert_eq!(reconciler.slots()[0].node_id, unkeyed_node);
        assert_eq!(reconciler.slots()[1].node_id, keyed_node);
    }
}
