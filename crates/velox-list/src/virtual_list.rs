use std::cell::{Cell, RefCell};

use velox_scene::{Layout, NodeId, NodeTree, Rect};

use crate::callbacks::ListCallbacks;
use crate::range::{compute_expanded, ExpandedRanges, ViewportRange};
use crate::scroll::ScrollAnchor;

struct CachedState {
    ranges: ExpandedRanges,
    viewport_height: f32,
}

pub struct VirtualList {
    scroll_offset: Cell<f32>,
    item_count: Cell<usize>,
    item_height: f32,
    cached_ranges: RefCell<Option<CachedState>>,
    callbacks: RefCell<ListCallbacks>,
}

impl VirtualList {
    pub fn new(item_height: f32, item_count: usize) -> Self {
        let safe_height = if item_height <= 0.0 { 1.0 } else { item_height };
        Self {
            scroll_offset: Cell::new(0.0),
            item_count: Cell::new(item_count),
            item_height: safe_height,
            cached_ranges: RefCell::new(None),
            callbacks: RefCell::new(ListCallbacks::default()),
        }
    }

    pub fn set_item_count(&self, count: usize) {
        self.item_count.set(count);
    }

    pub fn scroll_by(&self, delta: f32) {
        let content_height = self.item_height * self.item_count.get() as f32;
        let viewport_height = self
            .cached_ranges
            .borrow()
            .as_ref()
            .map(|c| c.viewport_height)
            .unwrap_or(0.0);
        let max_offset = (content_height - viewport_height).max(0.0);

        let new_offset = self.scroll_offset.get() + delta;
        self.scroll_offset.set(new_offset.clamp(0.0, max_offset));
    }

    pub fn scroll_to_index(&self, index: usize) {
        let clamped = index.min(self.item_count.get().saturating_sub(1));
        self.scroll_offset.set(clamped as f32 * self.item_height);
    }

    pub fn save_anchor(&self) -> ScrollAnchor {
        let offset = self.scroll_offset.get();
        let index = (offset / self.item_height).floor() as usize;
        let item_offset = offset - (index as f32 * self.item_height);
        ScrollAnchor {
            index,
            offset: item_offset,
        }
    }

    pub fn restore_anchor(&self, anchor: ScrollAnchor) {
        let offset = anchor.index as f32 * self.item_height + anchor.offset;
        self.scroll_offset.set(offset);
    }

    pub fn set_callbacks(&self, callbacks: ListCallbacks) {
        *self.callbacks.borrow_mut() = callbacks;
    }

    pub fn visible_range(&self) -> Option<ViewportRange> {
        self.cached_ranges
            .borrow()
            .as_ref()
            .map(|c| c.ranges.visible)
    }

    fn fire_callbacks(&self, old_ranges: Option<&ExpandedRanges>, new_ranges: &ExpandedRanges) {
        let callbacks = self.callbacks.borrow();

        let old_visible = old_ranges.map(|r| r.visible);
        if old_visible.as_ref() != Some(&new_ranges.visible) {
            if let Some(ref cb) = callbacks.on_visible_range_changed {
                cb(new_ranges.visible);
            }
        }

        let old_prefetch = old_ranges.map(|r| r.prefetch);
        if old_prefetch.as_ref() != Some(&new_ranges.prefetch) {
            if let Some(ref cb) = callbacks.on_prefetch_range_changed {
                cb(new_ranges.prefetch);
            }
        }

        if let Some(ref cb) = callbacks.on_item_visible {
            let old_vis = old_visible.unwrap_or(ViewportRange {
                start_index: 0,
                end_index: 0,
            });
            for i in new_ranges.visible.start_index..new_ranges.visible.end_index {
                if !old_vis.contains(i) {
                    cb(i);
                }
            }
        }

        if let Some(ref cb) = callbacks.on_item_hidden {
            if let Some(old_vis) = old_visible {
                for i in old_vis.start_index..old_vis.end_index {
                    if !new_ranges.visible.contains(i) {
                        cb(i);
                    }
                }
            }
        }
    }
}

impl Layout for VirtualList {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree) {
        let viewport_height = parent_rect.height;
        let count = self.item_count.get().min(children.len());
        let content_height = self.item_height * count as f32;
        let offset = self
            .scroll_offset
            .get()
            .clamp(0.0, (content_height - viewport_height).max(0.0));
        self.scroll_offset.set(offset);

        let first_visible = (offset / self.item_height).floor() as usize;
        let last_visible = ((offset + viewport_height) / self.item_height).ceil() as usize;
        let visible = ViewportRange {
            start_index: first_visible.min(count),
            end_index: last_visible.min(count),
        };

        let new_ranges = compute_expanded(visible, count, 1.0, 1.0);

        let old_ranges = self.cached_ranges.borrow();
        let old_expanded = old_ranges.as_ref().map(|c| &c.ranges);
        self.fire_callbacks(old_expanded, &new_ranges);
        drop(old_ranges);

        for (i, &child) in children.iter().enumerate() {
            if i < count && i >= visible.start_index && i < visible.end_index {
                let y = parent_rect.y + (i as f32 * self.item_height) - offset;
                tree.set_rect(
                    child,
                    Rect::new(parent_rect.x, y, parent_rect.width, self.item_height),
                );
                tree.set_visible(child, true);
            } else {
                tree.set_visible(child, false);
            }
        }

        *self.cached_ranges.borrow_mut() = Some(CachedState {
            ranges: new_ranges,
            viewport_height,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_scene::NodeTree;

    fn setup_tree(item_count: usize) -> (NodeTree, NodeId, Vec<NodeId>) {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 100.0));
        let children: Vec<NodeId> = (0..item_count).map(|_| tree.insert(Some(root))).collect();
        (tree, root, children)
    }

    #[test]
    fn layout_positions_visible_children() {
        let (mut tree, root, children) = setup_tree(10);
        let list = VirtualList::new(25.0, 10);

        let parent_rect = tree.rect(root).unwrap();
        list.compute(parent_rect, &children, &mut tree);

        let r0 = tree.rect(children[0]).unwrap();
        assert_eq!(r0.y, 0.0);
        assert_eq!(r0.height, 25.0);
        assert_eq!(r0.width, 200.0);

        let r1 = tree.rect(children[1]).unwrap();
        assert_eq!(r1.y, 25.0);

        assert_eq!(tree.is_visible(children[0]), Some(true));
        assert_eq!(tree.is_visible(children[3]), Some(true));
    }

    #[test]
    fn layout_hides_offscreen_children() {
        let (mut tree, root, children) = setup_tree(20);
        let list = VirtualList::new(25.0, 20);

        let parent_rect = tree.rect(root).unwrap();
        list.compute(parent_rect, &children, &mut tree);

        assert_eq!(tree.is_visible(children[0]), Some(true));
        assert_eq!(tree.is_visible(children[3]), Some(true));
        assert_eq!(tree.is_visible(children[5]), Some(false));
        assert_eq!(tree.is_visible(children[19]), Some(false));
    }

    #[test]
    fn scroll_by_clamps_offset() {
        let list = VirtualList::new(25.0, 10);
        list.scroll_by(-100.0);
        assert_eq!(list.scroll_offset.get(), 0.0);

        list.scroll_by(50.0);
        assert_eq!(list.scroll_offset.get(), 50.0);
    }

    #[test]
    fn scroll_to_index_sets_offset() {
        let list = VirtualList::new(25.0, 10);
        list.scroll_to_index(5);
        assert_eq!(list.scroll_offset.get(), 125.0);
    }

    #[test]
    fn scroll_to_index_clamps_to_last() {
        let list = VirtualList::new(25.0, 10);
        list.scroll_to_index(100);
        assert_eq!(list.scroll_offset.get(), 225.0);
    }

    #[test]
    fn save_and_restore_anchor() {
        let list = VirtualList::new(50.0, 20);
        list.scroll_by(75.0);

        let anchor = list.save_anchor();
        assert_eq!(anchor.index, 1);
        assert!((anchor.offset - 25.0).abs() < 0.01);

        list.scroll_by(100.0);
        list.restore_anchor(anchor);
        assert!((list.scroll_offset.get() - 75.0).abs() < 0.01);
    }

    #[test]
    fn visible_range_returns_cached() {
        let (mut tree, root, children) = setup_tree(10);
        let list = VirtualList::new(25.0, 10);

        assert!(list.visible_range().is_none());

        let parent_rect = tree.rect(root).unwrap();
        list.compute(parent_rect, &children, &mut tree);

        let range = list.visible_range().unwrap();
        assert_eq!(range.start_index, 0);
        assert_eq!(range.end_index, 4);
    }
}
