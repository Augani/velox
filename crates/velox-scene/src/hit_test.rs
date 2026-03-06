#[cfg(test)]
mod tests {
    use crate::geometry::{Point, Rect};
    use crate::tree::NodeTree;

    #[test]
    fn hit_test_returns_deepest_node() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));
        let grandchild = tree.insert(Some(child));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_rect(child, Rect::new(10.0, 10.0, 100.0, 100.0));
        tree.set_rect(grandchild, Rect::new(20.0, 20.0, 50.0, 50.0));

        let hit = tree.hit_test(Point::new(30.0, 30.0));
        assert_eq!(hit, Some(grandchild));
    }

    #[test]
    fn hit_test_returns_parent_when_miss_child() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_rect(child, Rect::new(50.0, 50.0, 50.0, 50.0));

        let hit = tree.hit_test(Point::new(5.0, 5.0));
        assert_eq!(hit, Some(root));
    }

    #[test]
    fn hit_test_returns_none_outside_root() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));

        let hit = tree.hit_test(Point::new(150.0, 150.0));
        assert_eq!(hit, None);
    }

    #[test]
    fn hit_test_skips_invisible_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_rect(child, Rect::new(10.0, 10.0, 100.0, 100.0));
        tree.set_visible(child, false);

        let hit = tree.hit_test(Point::new(50.0, 50.0));
        assert_eq!(hit, Some(root));
    }

    #[test]
    fn hit_test_skips_transparent_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_rect(child, Rect::new(10.0, 10.0, 100.0, 100.0));
        tree.set_hit_test_transparent(child, true);

        let hit = tree.hit_test(Point::new(50.0, 50.0));
        assert_eq!(hit, Some(root));
    }

    #[test]
    fn hit_test_last_child_has_priority() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child_a = tree.insert(Some(root));
        let child_b = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        tree.set_rect(child_a, Rect::new(10.0, 10.0, 100.0, 100.0));
        tree.set_rect(child_b, Rect::new(50.0, 50.0, 100.0, 100.0));

        let hit = tree.hit_test(Point::new(60.0, 60.0));
        assert_eq!(hit, Some(child_b));

        let _ = child_a;
    }

    #[test]
    fn hit_test_empty_tree() {
        let tree = NodeTree::new();
        let hit = tree.hit_test(Point::new(0.0, 0.0));
        assert_eq!(hit, None);
    }
}
