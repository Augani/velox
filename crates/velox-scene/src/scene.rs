use crate::focus::FocusState;
use crate::geometry::Point;
use crate::node::NodeId;
use crate::overlay::{OverlayId, OverlayStack};
use crate::paint::CommandList;
use crate::tree::NodeTree;

pub struct Scene {
    tree: NodeTree,
    overlay_stack: OverlayStack,
    focus: FocusState,
    command_list: CommandList,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            tree: NodeTree::new(),
            overlay_stack: OverlayStack::new(),
            focus: FocusState::new(),
            command_list: CommandList::new(),
        }
    }

    pub fn tree(&self) -> &NodeTree {
        &self.tree
    }

    pub fn tree_mut(&mut self) -> &mut NodeTree {
        &mut self.tree
    }

    pub fn overlay_stack(&self) -> &OverlayStack {
        &self.overlay_stack
    }

    pub fn overlay_stack_mut(&mut self) -> &mut OverlayStack {
        &mut self.overlay_stack
    }

    pub fn focus(&self) -> &FocusState {
        &self.focus
    }

    pub fn focus_mut(&mut self) -> &mut FocusState {
        &mut self.focus
    }

    pub fn push_overlay(&mut self) -> OverlayId {
        self.overlay_stack.push_overlay()
    }

    pub fn layout(&mut self) {
        self.tree.run_layout();
        self.overlay_stack
            .for_each_tree_mut(|tree| tree.run_layout());
    }

    pub fn paint(&mut self) {
        self.command_list.clear();
        self.tree.run_paint(&mut self.command_list);
        self.overlay_stack
            .for_each_tree_mut(|tree| tree.run_paint(&mut self.command_list));
    }

    pub fn hit_test(&self, point: Point) -> Option<NodeId> {
        if let Some((_overlay_id, node_id)) = self.overlay_stack.hit_test(point) {
            return Some(node_id);
        }
        self.tree.hit_test(point)
    }

    pub fn commands(&self) -> &CommandList {
        &self.command_list
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Direction, StackLayout};
    use crate::paint::Color;
    use crate::painter::Painter;

    struct TestPainter {
        color: Color,
    }

    impl Painter for TestPainter {
        fn paint(&self, rect: Rect, commands: &mut CommandList) {
            commands.fill_rect(rect, self.color);
        }
    }

    #[test]
    fn scene_layout_then_paint() {
        let mut scene = Scene::new();

        let root = scene.tree_mut().insert(None);
        let child = scene.tree_mut().insert(Some(root));

        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 200.0, 200.0));
        scene.tree_mut().set_layout(
            root,
            StackLayout {
                direction: Direction::Vertical,
                spacing: 0.0,
            },
        );
        scene.tree_mut().set_painter(
            root,
            TestPainter {
                color: Color::rgb(255, 0, 0),
            },
        );
        scene.tree_mut().set_painter(
            child,
            TestPainter {
                color: Color::rgb(0, 255, 0),
            },
        );

        scene.layout();
        scene.paint();

        assert!(!scene.commands().commands().is_empty());
    }

    #[test]
    fn scene_hit_test_checks_overlays_first() {
        let mut scene = Scene::new();

        let main_root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(main_root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let overlay_id = scene.push_overlay();
        let tree = scene
            .overlay_stack_mut()
            .overlay_tree_mut(overlay_id)
            .unwrap();
        let overlay_root = tree.insert(None);
        tree.set_rect(overlay_root, Rect::new(0.0, 0.0, 100.0, 100.0));

        let hit = scene.hit_test(Point::new(50.0, 50.0));
        assert_eq!(hit, Some(overlay_root));
    }

    #[test]
    fn scene_hit_test_falls_through_to_main_tree() {
        let mut scene = Scene::new();

        let main_root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(main_root, Rect::new(0.0, 0.0, 500.0, 500.0));

        let overlay_id = scene.push_overlay();
        let tree = scene
            .overlay_stack_mut()
            .overlay_tree_mut(overlay_id)
            .unwrap();
        let overlay_root = tree.insert(None);
        tree.set_rect(overlay_root, Rect::new(0.0, 0.0, 50.0, 50.0));

        let hit = scene.hit_test(Point::new(200.0, 200.0));
        assert_eq!(hit, Some(main_root));

        let _ = overlay_root;
    }

    #[test]
    fn scene_focus() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);

        scene.focus_mut().request_focus(root);
        assert_eq!(scene.focus().focused(), Some(root));
    }

    #[test]
    fn scene_paint_clears_and_rebuilds_commands() {
        let mut scene = Scene::new();
        let root = scene.tree_mut().insert(None);
        scene
            .tree_mut()
            .set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));
        scene.tree_mut().set_painter(
            root,
            TestPainter {
                color: Color::rgb(255, 0, 0),
            },
        );

        scene.paint();
        let first_count = scene.commands().commands().len();

        scene.paint();
        let second_count = scene.commands().commands().len();

        assert_eq!(first_count, second_count);
        assert!(first_count > 0);
    }
}
