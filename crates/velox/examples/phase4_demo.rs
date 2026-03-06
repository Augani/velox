use std::cell::RefCell;
use std::rc::Rc;

use velox::prelude::*;
use velox::scene::{
    ButtonState, Color, CommandList, EventContext, EventHandler, Key, KeyEvent, Modifiers,
    MouseButton, MouseEvent, PaddingLayout, Painter, PositionedGlyph,
};
use velox::text::{CursorDirection, EditableText, FontSystem, GlyphRasterizer};

struct TextInputState {
    editable: EditableText,
    font_system: FontSystem,
    rasterizer: GlyphRasterizer,
    focused: bool,
    cursor_visible: bool,
}

impl TextInputState {
    fn new() -> Self {
        let mut fs = FontSystem::new();
        let mut editable = EditableText::new(&mut fs, 16.0, 24.0, false);
        editable.set_size(&mut fs, 500.0, 40.0);
        editable.set_text(&mut fs, "Type here...");
        editable.select_all();
        Self {
            editable,
            font_system: fs,
            rasterizer: GlyphRasterizer::new(),
            focused: true,
            cursor_visible: true,
        }
    }
}

struct TextInputPainter {
    state: Rc<RefCell<TextInputState>>,
}

impl Painter for TextInputPainter {
    fn paint(&self, rect: Rect, commands: &mut CommandList) {
        let mut state = self.state.borrow_mut();
        let TextInputState {
            editable,
            font_system,
            rasterizer,
            focused,
            cursor_visible,
        } = &mut *state;

        commands.fill_rect(rect, Color::rgb(50, 50, 60));

        for sel_rect in editable.selection_rects() {
            commands.fill_rect(
                Rect::new(
                    rect.x + sel_rect.x,
                    rect.y + sel_rect.y,
                    sel_rect.width,
                    sel_rect.height,
                ),
                Color::rgba(80, 120, 200, 100),
            );
        }

        let mut glyphs = Vec::new();
        for run in editable.buffer().layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                if let Some(rasterized) = rasterizer.rasterize(font_system, physical.cache_key) {
                    if rasterized.width > 0 && rasterized.height > 0 {
                        commands.upload_glyph(
                            physical.cache_key,
                            rasterized.width,
                            rasterized.height,
                            rasterized.data,
                        );
                        glyphs.push(PositionedGlyph {
                            cache_key: physical.cache_key,
                            x: rect.x + physical.x as f32 + rasterized.left as f32,
                            y: rect.y + run.line_y + physical.y as f32 - rasterized.top as f32,
                            width: rasterized.width as f32,
                            height: rasterized.height as f32,
                        });
                    }
                }
            }
        }
        if !glyphs.is_empty() {
            commands.draw_glyphs(glyphs, Color::rgb(230, 230, 240));
        }

        if *focused && *cursor_visible {
            if let Some(cr) = editable.cursor_rect() {
                commands.fill_rect(
                    Rect::new(rect.x + cr.x, rect.y + cr.y, cr.width, cr.height),
                    Color::rgb(200, 200, 220),
                );
            }
        }
    }
}

struct TextInputEventHandler {
    state: Rc<RefCell<TextInputState>>,
}

impl EventHandler for TextInputEventHandler {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool {
        if !event.state.is_pressed() {
            return false;
        }
        let mut state = self.state.borrow_mut();
        let TextInputState {
            editable,
            font_system,
            cursor_visible,
            ..
        } = &mut *state;

        let is_cmd = cfg!(target_os = "macos") && event.modifiers.contains(Modifiers::SUPER)
            || !cfg!(target_os = "macos") && event.modifiers.contains(Modifiers::CTRL);

        match event.key {
            Key::A if is_cmd => editable.select_all(),
            Key::Z if is_cmd && event.modifiers.contains(Modifiers::SHIFT) => {
                editable.redo(font_system);
            }
            Key::Z if is_cmd => editable.undo(font_system),
            Key::C if is_cmd => {
                let text = editable.selected_text().to_owned();
                if !text.is_empty() {
                    ctx.clipboard_set(&text);
                }
            }
            Key::X if is_cmd => {
                let text = editable.selected_text().to_owned();
                if !text.is_empty() {
                    ctx.clipboard_set(&text);
                    editable.delete_backward(font_system);
                }
            }
            Key::V if is_cmd => {
                if let Some(text) = ctx.clipboard_get() {
                    let text = text.to_owned();
                    editable.insert_text(font_system, &text);
                }
            }
            Key::Backspace => editable.delete_backward(font_system),
            Key::Delete => editable.delete_forward(font_system),
            Key::ArrowLeft => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                editable.move_cursor(font_system, CursorDirection::Left, extend);
            }
            Key::ArrowRight => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                editable.move_cursor(font_system, CursorDirection::Right, extend);
            }
            Key::Home => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                editable.move_cursor(font_system, CursorDirection::Home, extend);
            }
            Key::End => {
                let extend = event.modifiers.contains(Modifiers::SHIFT);
                editable.move_cursor(font_system, CursorDirection::End, extend);
            }
            _ => {
                if let Some(ref text) = event.text {
                    for ch in text.chars() {
                        if !ch.is_control() {
                            editable.insert_char(font_system, ch);
                        }
                    }
                } else {
                    return false;
                }
            }
        }
        *cursor_visible = true;
        ctx.request_redraw();
        true
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        if event.button == MouseButton::Left && event.state == ButtonState::Pressed {
            let mut state = self.state.borrow_mut();
            let pos =
                state
                    .editable
                    .hit_test(&state.font_system, event.position.x, event.position.y);
            let TextInputState {
                editable,
                font_system,
                cursor_visible,
                ..
            } = &mut *state;
            editable.move_cursor_to(font_system, pos);
            *cursor_visible = true;
            ctx.request_redraw();
            return true;
        }
        false
    }

    fn handle_focus(&mut self, gained: bool) {
        let mut state = self.state.borrow_mut();
        state.focused = gained;
        state.cursor_visible = gained;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new()
        .name("Phase 4 Demo")
        .window(
            WindowConfig::new("main")
                .title("Velox — Text Editing")
                .size(800, 400),
        )
        .setup(|scene| {
            let root = scene.tree_mut().insert(None);
            scene
                .tree_mut()
                .set_rect(root, Rect::new(0.0, 0.0, 800.0, 400.0));
            scene.tree_mut().set_layout(
                root,
                PaddingLayout {
                    top: 50.0,
                    right: 50.0,
                    bottom: 50.0,
                    left: 50.0,
                },
            );

            let input = scene.tree_mut().insert(Some(root));
            scene
                .tree_mut()
                .set_rect(input, Rect::new(0.0, 0.0, 700.0, 40.0));

            let widget_state = Rc::new(RefCell::new(TextInputState::new()));

            scene.tree_mut().set_painter(
                input,
                TextInputPainter {
                    state: widget_state.clone(),
                },
            );
            scene.tree_mut().set_event_handler(
                input,
                TextInputEventHandler {
                    state: widget_state,
                },
            );

            scene.focus_mut().request_focus(input);
        })
        .run()
}
