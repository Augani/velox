use std::cell::{Cell, RefCell};

use velox_scene::{Layout, NodeId, NodeTree, Rect};

use crate::range::ViewportRange;

pub struct VirtualGrid {
    scroll_offset: Cell<f32>,
    item_count: Cell<usize>,
    column_count: usize,
    item_width: f32,
    item_height: f32,
    cached_visible: RefCell<Option<ViewportRange>>,
}

impl VirtualGrid {
    pub fn new(column_count: usize, item_width: f32, item_height: f32, item_count: usize) -> Self {
        let safe_cols = column_count.max(1);
        let safe_width = if item_width <= 0.0 { 1.0 } else { item_width };
        let safe_height = if item_height <= 0.0 { 1.0 } else { item_height };
        Self {
            scroll_offset: Cell::new(0.0),
            item_count: Cell::new(item_count),
            column_count: safe_cols,
            item_width: safe_width,
            item_height: safe_height,
            cached_visible: RefCell::new(None),
        }
    }

    pub fn set_item_count(&self, count: usize) {
        self.item_count.set(count);
    }

    pub fn scroll_by(&self, delta: f32) {
        let total_rows = self.row_count(self.item_count.get());
        let content_height = total_rows as f32 * self.item_height;
        let new_offset = self.scroll_offset.get() + delta;
        self.scroll_offset
            .set(new_offset.clamp(0.0, content_height));
    }

    pub fn visible_range(&self) -> Option<ViewportRange> {
        *self.cached_visible.borrow()
    }

    fn row_count(&self, total_items: usize) -> usize {
        total_items.div_ceil(self.column_count)
    }
}

impl Layout for VirtualGrid {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree) {
        let viewport_height = parent_rect.height;
        let count = self.item_count.get().min(children.len());
        let total_rows = self.row_count(count);
        let content_height = total_rows as f32 * self.item_height;
        let offset = self
            .scroll_offset
            .get()
            .clamp(0.0, (content_height - viewport_height).max(0.0));
        self.scroll_offset.set(offset);

        let first_visible_row = (offset / self.item_height).floor() as usize;
        let last_visible_row = ((offset + viewport_height) / self.item_height).ceil() as usize;

        let first_visible_index = first_visible_row * self.column_count;
        let last_visible_index = (last_visible_row * self.column_count).min(count);

        let visible = ViewportRange {
            start_index: first_visible_index.min(count),
            end_index: last_visible_index,
        };

        for (i, &child) in children.iter().enumerate() {
            if i < count && i >= visible.start_index && i < visible.end_index {
                let row = i / self.column_count;
                let col = i % self.column_count;
                let x = parent_rect.x + col as f32 * self.item_width;
                let y = parent_rect.y + row as f32 * self.item_height - offset;
                tree.set_rect(child, Rect::new(x, y, self.item_width, self.item_height));
                tree.set_visible(child, true);
            } else {
                tree.set_visible(child, false);
            }
        }

        *self.cached_visible.borrow_mut() = Some(visible);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velox_scene::NodeTree;

    fn setup_grid_tree(item_count: usize) -> (NodeTree, NodeId, Vec<NodeId>) {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 300.0, 100.0));
        let children: Vec<NodeId> = (0..item_count).map(|_| tree.insert(Some(root))).collect();
        (tree, root, children)
    }

    #[test]
    fn grid_positions_children_correctly() {
        let (mut tree, root, children) = setup_grid_tree(6);
        let grid = VirtualGrid::new(3, 100.0, 50.0, 6);

        let parent_rect = tree.rect(root).unwrap();
        grid.compute(parent_rect, &children, &mut tree);

        let r0 = tree.rect(children[0]).unwrap();
        assert_eq!(r0.x, 0.0);
        assert_eq!(r0.y, 0.0);
        assert_eq!(r0.width, 100.0);
        assert_eq!(r0.height, 50.0);

        let r1 = tree.rect(children[1]).unwrap();
        assert_eq!(r1.x, 100.0);
        assert_eq!(r1.y, 0.0);

        let r3 = tree.rect(children[3]).unwrap();
        assert_eq!(r3.x, 0.0);
        assert_eq!(r3.y, 50.0);
    }

    #[test]
    fn grid_hides_offscreen_items() {
        let (mut tree, root, children) = setup_grid_tree(12);
        let grid = VirtualGrid::new(3, 100.0, 50.0, 12);

        let parent_rect = tree.rect(root).unwrap();
        grid.compute(parent_rect, &children, &mut tree);

        assert_eq!(tree.is_visible(children[0]), Some(true));
        assert_eq!(tree.is_visible(children[5]), Some(true));
        assert_eq!(tree.is_visible(children[6]), Some(false));
        assert_eq!(tree.is_visible(children[11]), Some(false));
    }

    #[test]
    fn grid_visible_range() {
        let (mut tree, root, children) = setup_grid_tree(9);
        let grid = VirtualGrid::new(3, 100.0, 50.0, 9);

        let parent_rect = tree.rect(root).unwrap();
        grid.compute(parent_rect, &children, &mut tree);

        let range = grid.visible_range().unwrap();
        assert_eq!(range.start_index, 0);
        assert_eq!(range.end_index, 6);
    }
}
