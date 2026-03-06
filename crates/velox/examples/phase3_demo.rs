use velox::prelude::*;
use velox::scene::{Color, CommandList, Direction, PaddingLayout, Painter, StackLayout};

struct ColorBlock {
    color: Color,
}

impl Painter for ColorBlock {
    fn paint(&self, rect: Rect, commands: &mut CommandList) {
        commands.fill_rect(rect, self.color);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Phase 3 Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — GPU Rendering")
                .size(1200, 800),
        )
        .setup(|scene| {
            let root = scene.tree_mut().insert(None);
            scene
                .tree_mut()
                .set_rect(root, Rect::new(0.0, 0.0, 1200.0, 800.0));
            scene.tree_mut().set_layout(
                root,
                PaddingLayout {
                    top: 40.0,
                    right: 40.0,
                    bottom: 40.0,
                    left: 40.0,
                },
            );
            scene.tree_mut().set_painter(
                root,
                ColorBlock {
                    color: Color::rgb(30, 30, 35),
                },
            );

            let content = scene.tree_mut().insert(Some(root));
            scene.tree_mut().set_layout(
                content,
                StackLayout {
                    direction: Direction::Horizontal,
                    spacing: 20.0,
                },
            );

            let colors = [
                Color::rgb(220, 50, 50),
                Color::rgb(50, 180, 80),
                Color::rgb(60, 120, 220),
                Color::rgb(230, 180, 40),
                Color::rgb(160, 60, 210),
            ];

            for color in colors {
                let block = scene.tree_mut().insert(Some(content));
                scene
                    .tree_mut()
                    .set_rect(block, Rect::new(0.0, 0.0, 200.0, 300.0));
                scene.tree_mut().set_painter(block, ColorBlock { color });
            }

            let bottom_bar = scene.tree_mut().insert(Some(root));
            scene
                .tree_mut()
                .set_rect(bottom_bar, Rect::new(40.0, 700.0, 1120.0, 60.0));
            scene.tree_mut().set_painter(
                bottom_bar,
                ColorBlock {
                    color: Color::rgb(45, 45, 55),
                },
            );
        })
        .run()
}
