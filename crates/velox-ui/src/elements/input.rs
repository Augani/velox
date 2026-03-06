use crate::element::{
    AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
};
use crate::interactive::{EventHandlers, InteractiveElement};
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{PositionedGlyph, Rect};

type OnChangeCb = Box<dyn Fn(&str)>;

pub struct Input {
    style: Style,
    placeholder: Option<String>,
    multiline: bool,
    initial_value: Option<String>,
    on_change: Option<OnChangeCb>,
    handlers: EventHandlers,
}

pub fn input() -> Input {
    Input {
        style: Style::new(),
        placeholder: None,
        multiline: false,
        initial_value: None,
        on_change: None,
        handlers: EventHandlers::default(),
    }
}

impl Input {
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    pub fn on_change(mut self, cb: impl Fn(&str) + 'static) -> Self {
        self.on_change = Some(Box::new(cb));
        self
    }

    pub fn initial_value(mut self, text: impl Into<String>) -> Self {
        self.initial_value = Some(text.into());
        self
    }
}

impl Styled for Input {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style
    }
}

impl HasStyle for Input {
    fn get_style(&self) -> &Style {
        &self.style
    }
}

impl InteractiveElement for Input {
    fn handlers_mut(&mut self) -> &mut EventHandlers {
        &mut self.handlers
    }
}

#[derive(Default)]
pub struct InputState {
    editable: Option<velox_text::EditableText>,
    focused: bool,
    initialized: bool,
}

impl InputState {
    fn ensure_editable(
        &mut self,
        style: &Style,
        initial_value: &Option<String>,
        multiline: bool,
        font_system: &mut velox_text::FontSystem,
    ) {
        if self.editable.is_some() {
            return;
        }

        let font_size = style.font_size.unwrap_or(14.0);
        let line_height = style.line_height.unwrap_or(font_size * 1.2);
        let mut editable =
            velox_text::EditableText::new(font_system, font_size, line_height, multiline);

        if let Some(text) = initial_value.as_ref().filter(|_| !self.initialized) {
            editable.set_text(font_system, text);
            self.initialized = true;
        }

        self.editable = Some(editable);
    }

    pub fn insert_char(&mut self, font_system: &mut velox_text::FontSystem, ch: char) {
        if let Some(ref mut editable) = self.editable {
            editable.insert_char(font_system, ch);
        }
    }

    pub fn delete_backward(&mut self, font_system: &mut velox_text::FontSystem) {
        if let Some(ref mut editable) = self.editable {
            editable.delete_backward(font_system);
        }
    }

    pub fn text(&self) -> &str {
        match self.editable {
            Some(ref e) => e.text(),
            None => "",
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
}

impl Element for Input {
    type State = InputState;

    fn layout(
        &mut self,
        state: &mut InputState,
        _children: &[AnyElement],
        cx: &mut LayoutContext,
    ) -> LayoutRequest {
        state.ensure_editable(
            &self.style,
            &self.initial_value,
            self.multiline,
            cx.font_system(),
        );

        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(&mut self, state: &mut InputState, bounds: Rect, cx: &mut PaintContext) {
        if let Some(bg) = self.style.background {
            cx.commands().fill_rect(bounds, bg);
        }

        state.ensure_editable(
            &self.style,
            &self.initial_value,
            self.multiline,
            cx.font_system(),
        );

        let Some(ref editable) = state.editable else {
            return;
        };

        let text = editable.text();
        let is_empty = text.is_empty();

        let color = self
            .style
            .text_color
            .unwrap_or(velox_scene::Color::rgb(0, 0, 0));

        if !is_empty {
            let buffer = editable.buffer();
            let mut glyphs = Vec::new();
            let mut uploads = Vec::new();

            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((0.0, 0.0), 1.0);

                    let rasterized = cx
                        .glyph_rasterizer
                        .rasterize(cx.font_system, physical.cache_key);

                    if let Some(raster) =
                        rasterized.as_ref().filter(|r| r.width > 0 && r.height > 0)
                    {
                        uploads.push((
                            physical.cache_key,
                            raster.width,
                            raster.height,
                            raster.data.clone(),
                        ));
                    }

                    glyphs.push(PositionedGlyph {
                        cache_key: physical.cache_key,
                        x: bounds.x + physical.x as f32,
                        y: bounds.y + run.line_y + physical.y as f32,
                        width: glyph.w,
                        height: run.line_height,
                    });
                }
            }

            for (cache_key, width, height, data) in uploads {
                cx.commands().upload_glyph(cache_key, width, height, data);
            }

            if !glyphs.is_empty() {
                cx.commands().draw_glyphs(glyphs, color);
            }
        }

        if state.focused {
            if let Some(cursor_rect) = editable.cursor_rect() {
                let cursor_color = self
                    .style
                    .text_color
                    .unwrap_or(velox_scene::Color::rgb(0, 0, 0));
                cx.commands().fill_rect(
                    Rect::new(
                        bounds.x + cursor_rect.x,
                        bounds.y + cursor_rect.y,
                        1.0,
                        cursor_rect.height,
                    ),
                    cursor_color,
                );
            }

            for sel_rect in editable.selection_rects() {
                cx.commands().fill_rect(
                    Rect::new(
                        bounds.x + sel_rect.x,
                        bounds.y + sel_rect.y,
                        sel_rect.width,
                        sel_rect.height,
                    ),
                    velox_scene::Color::rgba(0, 120, 215, 80),
                );
            }
        }
    }
}

impl IntoElement for Input {
    type Element = Input;
    fn into_element(self) -> Input {
        self
    }
}

impl IntoAnyElement for Input {
    fn into_any_element(self) -> AnyElement {
        AnyElement::new(self, None, vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_creates_editable_on_layout() {
        let mut inp = input().initial_value("hello");
        let mut state = InputState::default();
        let mut taffy = taffy::TaffyTree::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut cx = LayoutContext {
            taffy: &mut taffy,
            font_system: &mut font_system,
        };
        inp.layout(&mut state, &[], &mut cx);
        assert!(state.editable.is_some());
        assert_eq!(state.text(), "hello");
    }

    #[test]
    fn insert_char_modifies_text() {
        let mut state = InputState::default();
        let mut font_system = velox_text::FontSystem::new();

        let style = Style::new();
        state.ensure_editable(&style, &None, false, &mut font_system);
        state.insert_char(&mut font_system, 'A');
        state.insert_char(&mut font_system, 'B');

        assert_eq!(state.text(), "AB");
    }

    #[test]
    fn delete_backward_removes_char() {
        let mut state = InputState::default();
        let mut font_system = velox_text::FontSystem::new();

        let style = Style::new();
        state.ensure_editable(&style, &None, false, &mut font_system);
        state.insert_char(&mut font_system, 'A');
        state.insert_char(&mut font_system, 'B');
        state.delete_backward(&mut font_system);

        assert_eq!(state.text(), "A");
    }

    #[test]
    fn paint_emits_draw_glyphs_with_text() {
        let mut inp = input().initial_value("Hi");
        let mut state = InputState::default();

        let mut taffy = taffy::TaffyTree::new();
        let mut font_system = velox_text::FontSystem::new();
        let mut lcx = LayoutContext {
            taffy: &mut taffy,
            font_system: &mut font_system,
        };
        inp.layout(&mut state, &[], &mut lcx);

        let theme = velox_style::Theme::light();
        let mut commands = velox_scene::CommandList::new();
        let mut fs = velox_text::FontSystem::new();
        let mut gr = velox_text::GlyphRasterizer::new();
        let mut cx = PaintContext {
            commands: &mut commands,
            theme: &theme,
            font_system: &mut fs,
            glyph_rasterizer: &mut gr,
            hovered_node: None,
            active_node: None,
        };

        inp.paint(&mut state, Rect::new(0.0, 0.0, 200.0, 30.0), &mut cx);

        let has_draw_glyphs = commands
            .commands()
            .iter()
            .any(|c| matches!(c, velox_scene::PaintCommand::DrawGlyphs { .. }));
        assert!(has_draw_glyphs);
    }

    #[test]
    fn on_change_callback_stored() {
        use std::cell::Cell;
        use std::rc::Rc;

        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        let inp = input().on_change(move |_text| {
            c.set(true);
        });
        assert!(inp.on_change.is_some());
    }
}
