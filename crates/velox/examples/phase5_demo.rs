use std::cell::RefCell;
use std::rc::Rc;

use velox::prelude::*;
use velox::scene::{
    ButtonState, Color, CommandList, EventContext, EventHandler, Key, KeyEvent, MouseButton,
    MouseEvent, Painter,
};
use velox::style::{Theme, ThemeColor, ThemeManager};

struct ThemeDemoState {
    manager: ThemeManager,
    dark: bool,
}

impl ThemeDemoState {
    fn new() -> Self {
        Self {
            manager: ThemeManager::new(Theme::light()),
            dark: false,
        }
    }

    fn theme(&self) -> Theme {
        self.manager.current()
    }

    fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        let next = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        self.manager.set_theme(next);
    }
}

struct ThemeDemoPainter {
    state: Rc<RefCell<ThemeDemoState>>,
}

impl Painter for ThemeDemoPainter {
    fn paint(&self, rect: Rect, commands: &mut CommandList) {
        let theme = self.state.borrow().theme();
        let palette = &theme.palette;

        commands.fill_rect(rect, to_scene_color(palette.background));

        let outer = theme.space.xl.value();
        let card = Rect::new(
            rect.x + outer,
            rect.y + outer,
            rect.width - outer * 2.0,
            rect.height - outer * 2.0,
        );
        commands.fill_rect(card, to_scene_color(palette.surface));

        let stroke = 2.0;
        commands.stroke_rect(card, to_scene_color(palette.surface_alt), stroke);

        let header_height = theme.space.xl.value() * 2.0;
        let header = Rect::new(card.x, card.y, card.width, header_height);
        commands.fill_rect(header, to_scene_color(palette.accent.with_alpha(36)));

        let chip_pad = theme.space.lg.value();
        let chip_w = 180.0;
        let chip_h = 52.0;
        let left_chip = Rect::new(
            card.x + chip_pad,
            card.y + header_height + chip_pad,
            chip_w,
            chip_h,
        );
        commands.fill_rect(left_chip, to_scene_color(palette.surface_alt));
        commands.stroke_rect(left_chip, to_scene_color(palette.accent.with_alpha(90)), 1.0);

        let right_chip = Rect::new(
            card.x + card.width - chip_w - chip_pad,
            card.y + header_height + chip_pad,
            chip_w,
            chip_h,
        );
        commands.fill_rect(right_chip, to_scene_color(palette.selection));
        commands.stroke_rect(right_chip, to_scene_color(palette.accent), 1.5);
    }
}

struct ThemeDemoHandler {
    state: Rc<RefCell<ThemeDemoState>>,
}

impl EventHandler for ThemeDemoHandler {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool {
        if event.state.is_pressed() && event.key == Key::T {
            self.state.borrow_mut().toggle_theme();
            ctx.request_redraw();
            return true;
        }
        false
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
            self.state.borrow_mut().toggle_theme();
            ctx.request_redraw();
            return true;
        }
        false
    }
}

fn to_scene_color(color: ThemeColor) -> Color {
    Color::rgba(color.r, color.g, color.b, color.a)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Phase 5 Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — Theme Tokens")
                .size(900, 520),
        )
        .setup(|scene| {
            let root = scene.tree_mut().insert(None);
            scene
                .tree_mut()
                .set_rect(root, Rect::new(0.0, 0.0, 900.0, 520.0));

            let shared = Rc::new(RefCell::new(ThemeDemoState::new()));

            scene
                .tree_mut()
                .set_painter(root, ThemeDemoPainter { state: shared.clone() });
            scene
                .tree_mut()
                .set_event_handler(root, ThemeDemoHandler { state: shared });
            scene.focus_mut().request_focus(root);
        })
        .run()
}
