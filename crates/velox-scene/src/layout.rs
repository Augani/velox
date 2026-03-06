use crate::geometry::Rect;
use crate::node::NodeId;
use crate::tree::NodeTree;

pub trait Layout {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

pub struct StackLayout {
    pub direction: Direction,
    pub spacing: f32,
}

impl Layout for StackLayout {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree) {
        let count = children.len();
        if count == 0 {
            return;
        }

        let total_spacing = self.spacing * (count as f32 - 1.0);

        match self.direction {
            Direction::Vertical => {
                let child_height = (parent_rect.height - total_spacing) / count as f32;
                for (i, &child) in children.iter().enumerate() {
                    let y = parent_rect.y + (child_height + self.spacing) * i as f32;
                    tree.set_rect(
                        child,
                        Rect::new(parent_rect.x, y, parent_rect.width, child_height),
                    );
                }
            }
            Direction::Horizontal => {
                let child_width = (parent_rect.width - total_spacing) / count as f32;
                for (i, &child) in children.iter().enumerate() {
                    let x = parent_rect.x + (child_width + self.spacing) * i as f32;
                    tree.set_rect(
                        child,
                        Rect::new(x, parent_rect.y, child_width, parent_rect.height),
                    );
                }
            }
        }
    }
}

pub struct PaddingLayout {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Layout for PaddingLayout {
    fn compute(&self, parent_rect: Rect, children: &[NodeId], tree: &mut NodeTree) {
        if children.is_empty() {
            return;
        }
        let child_rect = Rect::new(
            parent_rect.x + self.left,
            parent_rect.y + self.top,
            parent_rect.width - self.left - self.right,
            parent_rect.height - self.top - self.bottom,
        );
        tree.set_rect(children[0], child_rect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_layout_vertical() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let c1 = tree.insert(Some(root));
        let c2 = tree.insert(Some(root));
        let c3 = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 300.0));
        tree.set_layout(
            root,
            StackLayout {
                direction: Direction::Vertical,
                spacing: 0.0,
            },
        );

        tree.run_layout();

        let r1 = tree.rect(c1).unwrap();
        let r2 = tree.rect(c2).unwrap();
        let r3 = tree.rect(c3).unwrap();

        assert_eq!(r1.width, 100.0);
        assert_eq!(r2.width, 100.0);
        assert_eq!(r3.width, 100.0);

        assert_eq!(r1.y, 0.0);
        assert_eq!(r2.y, 100.0);
        assert_eq!(r3.y, 200.0);

        assert_eq!(r1.height, 100.0);
    }

    #[test]
    fn stack_layout_horizontal() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let c1 = tree.insert(Some(root));
        let c2 = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 50.0));
        tree.set_layout(
            root,
            StackLayout {
                direction: Direction::Horizontal,
                spacing: 0.0,
            },
        );

        tree.run_layout();

        let r1 = tree.rect(c1).unwrap();
        let r2 = tree.rect(c2).unwrap();

        assert_eq!(r1.height, 50.0);
        assert_eq!(r2.height, 50.0);

        assert_eq!(r1.x, 0.0);
        assert_eq!(r2.x, 100.0);

        assert_eq!(r1.width, 100.0);
    }

    #[test]
    fn padding_layout() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_layout(
            root,
            PaddingLayout {
                top: 10.0,
                right: 20.0,
                bottom: 30.0,
                left: 40.0,
            },
        );

        tree.run_layout();

        let r = tree.rect(child).unwrap();
        assert_eq!(r.x, 40.0);
        assert_eq!(r.y, 10.0);
        assert_eq!(r.width, 140.0);
        assert_eq!(r.height, 160.0);
    }

    #[test]
    fn layout_only_visits_dirty_subtrees() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_layout(
            root,
            StackLayout {
                direction: Direction::Vertical,
                spacing: 0.0,
            },
        );

        tree.run_layout();

        let after_layout = tree.rect(child).unwrap();
        assert_eq!(after_layout, Rect::new(0.0, 0.0, 200.0, 200.0));

        tree.set_rect(child, Rect::new(5.0, 5.0, 10.0, 10.0));
        tree.clear_dirty(root);

        tree.run_layout();

        let after_second = tree.rect(child).unwrap();
        assert_eq!(after_second, Rect::new(5.0, 5.0, 10.0, 10.0));
    }
}
