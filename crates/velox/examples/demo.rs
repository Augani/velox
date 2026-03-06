use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use velox::animation::{Easing, SpringConfig, Tween};
use velox::list::VirtualList;
use velox::media::{DecodedImage, PixelFormat};
use velox::prelude::*;
use velox::scene::{
    ButtonState, Color, CommandList, EventContext, EventHandler, Key, KeyEvent, MouseButton,
    MouseEvent, Painter, ScrollEvent, TextureId,
};
use velox::storage::SettingsStore;
use velox::style::{Theme, ThemeManager};

const WINDOW_W: f32 = 1200.0;
const WINDOW_H: f32 = 800.0;
const SIDEBAR_W: f32 = 300.0;
const ITEM_HEIGHT: f32 = 48.0;
const ITEM_COUNT: usize = 1000;

struct DemoState {
    theme_manager: ThemeManager,
    dark_mode: bool,
    settings: SettingsStore,
    accent_tween: Tween<f32>,
    sidebar_spring: velox::animation::Spring<f32>,
    sidebar_visible: bool,
    list: VirtualList,
    decoded_image: Option<velox::media::DecodedImage>,
}

impl DemoState {
    fn new(theme_manager: ThemeManager, settings: SettingsStore) -> Self {
        let dark_mode: bool = settings.get("dark_mode").unwrap_or(false);
        if dark_mode {
            theme_manager.set_theme(dark_theme());
        }

        let sidebar_visible: bool = settings.get("sidebar_visible").unwrap_or(true);
        let sidebar_target = if sidebar_visible { SIDEBAR_W } else { 0.0 };

        let decoded_image = Some(generate_gradient_image(64, 64));

        Self {
            theme_manager,
            dark_mode,
            settings,
            accent_tween: Tween::new(0.0_f32, 1.0, Duration::from_millis(600), Easing::InOutCubic),
            sidebar_spring: {
                let mut spring = velox::animation::Spring::new(
                    sidebar_target,
                    SpringConfig {
                        stiffness: 200.0,
                        damping: 22.0,
                        mass: 1.0,
                        rest_threshold: 0.5,
                    },
                );
                spring.set_target(sidebar_target);
                spring
            },
            sidebar_visible,
            list: VirtualList::new(ITEM_HEIGHT, ITEM_COUNT),
            decoded_image,
        }
    }

    fn toggle_theme(&mut self) {
        self.dark_mode = !self.dark_mode;
        let next = if self.dark_mode {
            dark_theme()
        } else {
            Theme::generated_default()
        };
        self.theme_manager.set_theme(next);
        self.accent_tween =
            Tween::new(0.0_f32, 1.0, Duration::from_millis(600), Easing::InOutCubic);
        let _ = self.settings.set("dark_mode", &self.dark_mode);
    }

    fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
        let target = if self.sidebar_visible { SIDEBAR_W } else { 0.0 };
        self.sidebar_spring.set_target(target);
        let _ = self.settings.set("sidebar_visible", &self.sidebar_visible);
    }

    fn theme(&self) -> Theme {
        self.theme_manager.current()
    }

    fn sidebar_width(&mut self) -> f32 {
        self.sidebar_spring.advance(Duration::ZERO)
    }
}

fn to_color(tc: velox::style::ThemeColor) -> Color {
    Color::rgba(tc.r, tc.g, tc.b, tc.a)
}

struct DemoPainter {
    state: Rc<RefCell<DemoState>>,
}

impl Painter for DemoPainter {
    fn paint(&self, rect: Rect, commands: &mut CommandList) {
        let mut state = self.state.borrow_mut();
        let theme = state.theme();
        let palette = &theme.palette;

        commands.fill_rect(rect, to_color(palette.background));

        let sidebar_w = state.sidebar_width();
        if sidebar_w > 1.0 {
            let sidebar_rect = Rect::new(rect.x, rect.y, sidebar_w, rect.height);
            commands.fill_rect(sidebar_rect, to_color(palette.surface));

            let header_h = 56.0;
            let header_rect = Rect::new(rect.x, rect.y, sidebar_w, header_h);
            commands.fill_rect(header_rect, to_color(palette.accent.with_alpha(40)));

            let divider = Rect::new(rect.x + sidebar_w - 1.0, rect.y, 1.0, rect.height);
            commands.fill_rect(divider, to_color(palette.surface_alt));

            let visible_range = state.list.visible_range();
            if let Some(range) = visible_range {
                for i in range.start_index..range.end_index {
                    let y = rect.y + header_h + (i as f32 * ITEM_HEIGHT)
                        - state.list.save_anchor().index as f32 * ITEM_HEIGHT
                        - state.list.save_anchor().offset;
                    if y + ITEM_HEIGHT < rect.y || y > rect.y + rect.height {
                        continue;
                    }
                    let item_rect = Rect::new(rect.x + 8.0, y, sidebar_w - 16.0, ITEM_HEIGHT - 2.0);
                    let item_color = if i % 2 == 0 {
                        to_color(palette.surface_alt)
                    } else {
                        to_color(palette.surface)
                    };
                    commands.fill_rect(item_rect, item_color);

                    let indicator = Rect::new(rect.x + 12.0, y + 14.0, 20.0, 20.0);
                    let hue = (i as f32 / ITEM_COUNT as f32 * 360.0) % 360.0;
                    commands.fill_rect(indicator, hue_to_color(hue));
                }
            }
        }

        let content_x = rect.x + sidebar_w;
        let content_w = rect.width - sidebar_w;
        if content_w > 10.0 {
            let content_rect = Rect::new(content_x, rect.y, content_w, rect.height);
            commands.fill_rect(content_rect, to_color(palette.background));

            let card_margin = theme.space.xl.value();
            let card_rect = Rect::new(
                content_x + card_margin,
                rect.y + card_margin,
                content_w - card_margin * 2.0,
                200.0,
            );
            commands.fill_rect(card_rect, to_color(palette.surface));
            commands.stroke_rect(card_rect, to_color(palette.surface_alt), 1.0);

            let tween_progress = state.accent_tween.value();
            let bar_width = (content_w - card_margin * 4.0) * tween_progress;
            let bar_rect = Rect::new(
                card_rect.x + card_margin,
                card_rect.y + card_margin,
                bar_width.max(0.0),
                12.0,
            );
            commands.fill_rect(bar_rect, to_color(palette.accent));

            if state.decoded_image.is_some() {
                let img_rect = Rect::new(
                    card_rect.x + card_margin,
                    card_rect.y + card_margin + 24.0,
                    64.0,
                    64.0,
                );
                commands.draw_image(TextureId(0), Rect::new(0.0, 0.0, 64.0, 64.0), img_rect, 1.0);
            }

            let info_y = card_rect.y + card_rect.height + card_margin;
            let chip_w = 140.0;
            let chip_h = 40.0;
            let chip_gap = theme.space.md.value();

            let theme_chip = Rect::new(content_x + card_margin, info_y, chip_w, chip_h);
            let chip_color = if state.dark_mode {
                to_color(palette.accent)
            } else {
                to_color(palette.selection)
            };
            commands.fill_rect(theme_chip, chip_color);

            let sidebar_chip = Rect::new(
                content_x + card_margin + chip_w + chip_gap,
                info_y,
                chip_w,
                chip_h,
            );
            let sidebar_chip_color = if state.sidebar_visible {
                to_color(palette.accent.with_alpha(128))
            } else {
                to_color(palette.surface_alt)
            };
            commands.fill_rect(sidebar_chip, sidebar_chip_color);
        }
    }
}

struct DemoHandler {
    state: Rc<RefCell<DemoState>>,
}

impl EventHandler for DemoHandler {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool {
        if !event.state.is_pressed() {
            return false;
        }
        match event.key {
            Key::T => {
                self.state.borrow_mut().toggle_theme();
                ctx.request_redraw();
                true
            }
            Key::S => {
                self.state.borrow_mut().toggle_sidebar();
                ctx.request_redraw();
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
            self.state.borrow_mut().toggle_theme();
            ctx.request_redraw();
            return true;
        }
        false
    }

    fn handle_scroll(&mut self, event: &ScrollEvent, ctx: &mut EventContext) -> bool {
        self.state.borrow_mut().list.scroll_by(-event.delta_y);
        ctx.request_redraw();
        true
    }
}

fn hue_to_color(hue: f32) -> Color {
    let h = (hue % 360.0) / 60.0;
    let x = (1.0 - (h % 2.0 - 1.0).abs()) * 255.0;
    let c = 255.0_f32;
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::rgb(r as u8, g as u8, b as u8)
}

fn generate_gradient_image(width: u32, height: u32) -> DecodedImage {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let r = (x as f32 / width as f32 * 255.0) as u8;
            let g = (y as f32 / height as f32 * 255.0) as u8;
            let b = 128u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    DecodedImage {
        width,
        height,
        format: PixelFormat::Rgba8,
        data: rgba,
    }
}

fn dark_theme() -> Theme {
    velox_style::theme! {
        name: "dark",
        palette: {
            background: [22, 22, 30, 255],
            surface: [32, 32, 42, 255],
            surface_alt: [42, 42, 54, 255],
            text_primary: [230, 230, 240, 255],
            text_muted: [140, 140, 160, 255],
            accent: [100, 140, 255, 255],
            accent_hover: [130, 165, 255, 255],
            selection: [100, 140, 255, 80],
        },
        space: { xs: 2.0, sm: 4.0, md: 8.0, lg: 12.0, xl: 20.0 },
        radius: { sm: 4.0, md: 8.0, lg: 12.0 },
        typography: { body: 14.0, heading: 20.0, mono: 13.0 },
    }
}

fn settings_path() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("velox_demo");
    dir.join("settings.toml")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ThemeManager::new(Theme::generated_default());
    let setup_manager = manager.clone();

    App::new()
        .name("Velox Demo")
        .power_policy(PowerPolicy::Adaptive)
        .theme_manager(manager)
        .window(
            WindowConfig::new("main")
                .title("Velox Demo — Animation + VirtualList + Media + Storage")
                .size(WINDOW_W as u32, WINDOW_H as u32)
                .min_size(600, 400),
        )
        .setup(move |scene| {
            let settings =
                SettingsStore::open(settings_path()).expect("Failed to open settings store");
            let state = Rc::new(RefCell::new(DemoState::new(setup_manager, settings)));

            let root = scene.tree_mut().insert(None);
            scene
                .tree_mut()
                .set_rect(root, Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H));

            scene.tree_mut().set_painter(
                root,
                DemoPainter {
                    state: state.clone(),
                },
            );
            scene
                .tree_mut()
                .set_event_handler(root, DemoHandler { state });
            scene.focus_mut().request_focus(root);
        })
        .run()
}
