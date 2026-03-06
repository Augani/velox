use crate::geometry::Rect;
use crate::paint::CommandList;

pub trait Painter {
    fn paint(&self, rect: Rect, commands: &mut CommandList);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::Color;
    use crate::tree::NodeTree;

    struct FillPainter {
        color: Color,
    }

    impl Painter for FillPainter {
        fn paint(&self, rect: Rect, commands: &mut CommandList) {
            commands.fill_rect(rect, self.color);
        }
    }

    #[test]
    fn paint_pass_collects_commands() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 50.0));
        tree.set_painter(
            root,
            FillPainter {
                color: Color::rgb(255, 0, 0),
            },
        );
        tree.set_painter(
            child,
            FillPainter {
                color: Color::rgb(0, 255, 0),
            },
        );

        let mut commands = CommandList::new();
        tree.run_paint(&mut commands);

        let cmds = commands.commands();
        assert!(cmds.len() >= 4);
        assert!(matches!(cmds[0], crate::paint::PaintCommand::PushClip(_)));
        assert!(matches!(
            cmds[1],
            crate::paint::PaintCommand::FillRect { .. }
        ));
    }

    #[test]
    fn paint_skips_invisible_nodes() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));
        tree.set_painter(
            root,
            FillPainter {
                color: Color::rgb(255, 0, 0),
            },
        );
        tree.set_visible(root, false);

        let mut commands = CommandList::new();
        tree.run_paint(&mut commands);

        assert!(commands.commands().is_empty());
    }

    #[test]
    fn paint_clears_dirty_flags() {
        let mut tree = NodeTree::new();
        let root = tree.insert(None);
        let child = tree.insert(Some(root));

        tree.set_rect(root, Rect::new(0.0, 0.0, 100.0, 100.0));
        tree.set_rect(child, Rect::new(10.0, 10.0, 50.0, 50.0));
        tree.set_painter(
            root,
            FillPainter {
                color: Color::rgb(255, 0, 0),
            },
        );

        assert!(tree.get(root).unwrap().paint_dirty);
        assert!(tree.get(child).unwrap().paint_dirty);

        let mut commands = CommandList::new();
        tree.run_paint(&mut commands);

        assert!(!tree.get(root).unwrap().paint_dirty);
        assert!(!tree.get(child).unwrap().paint_dirty);
    }
}
