use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityProps, AccessibleElement};
use crate::element::{
    AccessibilityAction, AccessibilityInfo, AnyElement, Element, HasStyle, IntoElement,
    LayoutContext, LayoutRequest, PaintContext,
};
use crate::interactive::{EventHandlers, InteractiveElement};
use crate::length::Length;
use crate::parent::IntoAnyElement;
use crate::style::Style;
use crate::styled::Styled;
use velox_scene::{Key, Modifiers, PositionedGlyph, Rect};
use velox_text::CursorDirection;

fn length_px(len: Option<Length>) -> f32 {
    match len {
        Some(Length::Px(v)) => v,
        _ => 0.0,
    }
}

type OnChangeCb = Box<dyn Fn(&str)>;

enum PendingInput {
    Text(String),
    Backspace,
    Delete,
    MoveCursor(CursorDirection, bool),
    SetText(String),
    ReplaceSelectedText(String),
    SelectAll,
    SetSelection(velox_text::TextSelection),
}

struct InputInner {
    editable: Option<velox_text::EditableText>,
    focused: bool,
    initialized: bool,
    pending: Vec<PendingInput>,
    on_change: Option<OnChangeCb>,
}

#[derive(Clone)]
pub struct InputHandle {
    inner: Rc<RefCell<InputInner>>,
}

impl InputHandle {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(InputInner {
                editable: None,
                focused: false,
                initialized: false,
                pending: Vec::new(),
                on_change: None,
            })),
        }
    }

    pub fn text(&self) -> String {
        let inner = self.inner.borrow();
        match inner.editable {
            Some(ref e) => e.text().to_string(),
            None => String::new(),
        }
    }

    pub fn is_focused(&self) -> bool {
        self.inner.borrow().focused
    }

    pub fn selection(&self) -> velox_text::TextSelection {
        self.inner
            .borrow()
            .editable
            .as_ref()
            .map(|editable| editable.selection())
            .unwrap_or_default()
    }

    pub fn set_text(&self, text: impl Into<String>) {
        self.inner
            .borrow_mut()
            .pending
            .push(PendingInput::SetText(text.into()));
    }

    pub fn replace_selected_text(&self, text: impl Into<String>) {
        self.inner
            .borrow_mut()
            .pending
            .push(PendingInput::ReplaceSelectedText(text.into()));
    }

    pub fn select_all(&self) {
        self.inner
            .borrow_mut()
            .pending
            .push(PendingInput::SelectAll);
    }

    pub fn set_selection(&self, selection: velox_text::TextSelection) {
        self.inner
            .borrow_mut()
            .pending
            .push(PendingInput::SetSelection(selection));
    }
}

impl Default for InputHandle {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Input {
    style: Style,
    accessibility: AccessibilityProps,
    placeholder: Option<String>,
    multiline: bool,
    initial_value: Option<String>,
    on_change: Option<OnChangeCb>,
    handlers: EventHandlers,
    handle: Option<InputHandle>,
}

pub fn input() -> Input {
    let mut style = Style::new();
    style.cursor = Some(crate::style::CursorStyle::Text);
    Input {
        style,
        accessibility: AccessibilityProps::default(),
        placeholder: None,
        multiline: false,
        initial_value: None,
        on_change: None,
        handlers: EventHandlers::default(),
        handle: None,
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

    pub fn handle(mut self, handle: InputHandle) -> Self {
        self.handle = Some(handle);
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

impl AccessibleElement for Input {
    fn accessibility_props_mut(&mut self) -> &mut AccessibilityProps {
        &mut self.accessibility
    }
}

#[derive(Default)]
pub struct InputState;

impl Element for Input {
    type State = InputState;

    fn take_handlers(&mut self) -> EventHandlers {
        let mut handlers = std::mem::take(&mut self.handlers);

        if let Some(ref handle) = self.handle {
            let h = handle.inner.clone();
            let multiline = self.multiline;
            handlers.on_key_down = Some(Rc::new(move |event: &velox_scene::KeyEvent| {
                let mut inner = h.borrow_mut();
                let shift = event.modifiers.contains(Modifiers::SHIFT);

                match event.key {
                    Key::Backspace => inner.pending.push(PendingInput::Backspace),
                    Key::Delete => inner.pending.push(PendingInput::Delete),
                    Key::ArrowLeft => {
                        inner
                            .pending
                            .push(PendingInput::MoveCursor(CursorDirection::Left, shift));
                    }
                    Key::ArrowRight => {
                        inner
                            .pending
                            .push(PendingInput::MoveCursor(CursorDirection::Right, shift));
                    }
                    Key::Home => {
                        inner
                            .pending
                            .push(PendingInput::MoveCursor(CursorDirection::Home, shift));
                    }
                    Key::End => {
                        inner
                            .pending
                            .push(PendingInput::MoveCursor(CursorDirection::End, shift));
                    }
                    Key::Enter => {
                        if multiline {
                            inner.pending.push(PendingInput::Text("\n".to_string()));
                        }
                    }
                    _ => {
                        if let Some(ref text) = event.text {
                            for ch in text.chars() {
                                if !ch.is_control() {
                                    inner.pending.push(PendingInput::Text(ch.to_string()));
                                }
                            }
                        }
                    }
                }
            }));

            let h2 = handle.inner.clone();
            handlers.on_focus = Some(Rc::new(move |focused: bool| {
                h2.borrow_mut().focused = focused;
            }));

            if self.on_change.is_some() {
                let cb = self.on_change.take().unwrap();
                handle.inner.borrow_mut().on_change = Some(cb);
            }
        }

        handlers
    }

    fn accessibility(
        &mut self,
        _state: &mut InputState,
        _children: &[AnyElement],
    ) -> AccessibilityInfo {
        let (current_text, selection, text_runs) = if let Some(handle) = self.handle.as_ref() {
            let inner = handle.inner.borrow();
            let current_text = inner
                .editable
                .as_ref()
                .map(|editable| editable.text().to_owned())
                .or_else(|| self.initial_value.clone());
            let selection = inner.editable.as_ref().map(|editable| editable.selection());
            let text_runs = inner
                .editable
                .as_ref()
                .map(|editable| accessibility_text_runs(editable, &self.style))
                .unwrap_or_default();
            (current_text, selection, text_runs)
        } else {
            (self.initial_value.clone(), None, Vec::new())
        };
        let mut node = self.accessibility.resolve(
            velox_scene::AccessibilityRole::TextInput,
            self.placeholder.clone(),
            current_text.filter(|value| !value.is_empty()),
            false,
        );
        node = node.supports_text_input_actions();
        node.text_selection = selection.map(|selection| velox_scene::AccessibilityTextSelection {
            anchor: selection.anchor,
            focus: selection.focus,
        });
        node.text_runs = text_runs.clone();
        AccessibilityInfo {
            node: Some(node),
            text_content: None,
            text_runs,
        }
    }

    fn handle_accessibility_action(
        &mut self,
        _state: &mut InputState,
        action: &AccessibilityAction,
    ) -> bool {
        let Some(handle) = self.handle.as_ref() else {
            return false;
        };

        match action {
            AccessibilityAction::SetValue(value) => {
                handle.set_text(value.clone());
                true
            }
            AccessibilityAction::ReplaceSelectedText(value) => {
                handle.replace_selected_text(value.clone());
                true
            }
            AccessibilityAction::SetTextSelection(selection) => {
                handle.set_selection(velox_text::TextSelection {
                    anchor: selection.anchor,
                    focus: selection.focus,
                });
                true
            }
        }
    }

    fn layout(
        &mut self,
        _state: &mut InputState,
        _children: &[AnyElement],
        cx: &mut LayoutContext,
    ) -> LayoutRequest {
        if let Some(ref handle) = self.handle {
            let mut inner = handle.inner.borrow_mut();
            if inner.editable.is_none() {
                let font_size = self.style.font_size.unwrap_or(14.0);
                let line_height = self.style.line_height.unwrap_or(font_size * 1.2);
                let mut editable = velox_text::EditableText::new(
                    cx.font_system(),
                    font_size,
                    line_height,
                    self.multiline,
                );
                if let Some(ref text) = self.initial_value
                    && !inner.initialized {
                        editable.set_text(cx.font_system(), text);
                        inner.initialized = true;
                    }
                inner.editable = Some(editable);
            }
        }

        LayoutRequest {
            taffy_style: crate::layout_engine::convert_style(&self.style),
        }
    }

    fn paint(&mut self, _state: &mut InputState, bounds: Rect, cx: &mut PaintContext) {
        let corner_radius = self.style.border_radius_tl.unwrap_or(0.0);
        if let Some(bg) = self.style.background {
            if corner_radius > 0.0 {
                cx.commands().fill_rounded_rect(bounds, bg, corner_radius);
            } else {
                cx.commands().fill_rect(bounds, bg);
            }
        }

        let Some(ref handle) = self.handle else {
            self.paint_placeholder(bounds, cx);
            return;
        };

        let mut inner = handle.inner.borrow_mut();

        if inner.editable.is_none() {
            drop(inner);
            self.paint_placeholder(bounds, cx);
            return;
        }

        let pending: Vec<PendingInput> = inner.pending.drain(..).collect();
        let mut changed = false;
        if let Some(ref mut editable) = inner.editable {
            for event in pending {
                match event {
                    PendingInput::Text(text) => {
                        for ch in text.chars() {
                            editable.insert_char(cx.font_system(), ch);
                        }
                        changed = true;
                    }
                    PendingInput::SetText(text) => {
                        editable.set_text(cx.font_system(), &text);
                        changed = true;
                    }
                    PendingInput::ReplaceSelectedText(text) => {
                        editable.insert_text(cx.font_system(), &text);
                        changed = true;
                    }
                    PendingInput::Backspace => {
                        editable.delete_backward(cx.font_system());
                        changed = true;
                    }
                    PendingInput::Delete => {
                        editable.delete_forward(cx.font_system());
                        changed = true;
                    }
                    PendingInput::MoveCursor(dir, extend) => {
                        editable.move_cursor(cx.font_system(), dir, extend);
                    }
                    PendingInput::SelectAll => {
                        editable.select_all();
                    }
                    PendingInput::SetSelection(selection) => {
                        editable.set_selection(selection);
                    }
                }
            }
        }

        if changed
            && let Some(ref on_change) = inner.on_change
                && let Some(ref editable) = inner.editable {
                    on_change(editable.text());
                }

        let is_empty = inner
            .editable
            .as_ref()
            .is_none_or(|e| e.text().is_empty());
        let focused = inner.focused;

        if is_empty {
            drop(inner);
            if !focused {
                self.paint_placeholder(bounds, cx);
            } else {
                self.paint_cursor_at_origin(bounds, cx);
                self.paint_placeholder(bounds, cx);
            }
            return;
        }

        let color = self
            .style
            .text_color
            .unwrap_or(velox_scene::Color::rgb(0, 0, 0));

        let sf = cx.scale_factor();
        let padding_x = length_px(self.style.padding_left);
        let padding_y = length_px(self.style.padding_top);
        let mut glyphs = Vec::new();

        if let Some(ref editable) = inner.editable {
            let buffer = editable.buffer();
            for run in buffer.layout_runs() {
                for glyph in run.glyphs.iter() {
                    let physical = glyph.physical((0.0, 0.0), sf);

                    let rasterized = cx
                        .glyph_rasterizer
                        .rasterize(cx.font_system, physical.cache_key);

                    let Some(raster) = rasterized else {
                        continue;
                    };
                    if raster.width == 0 || raster.height == 0 {
                        continue;
                    }

                    cx.commands().upload_glyph(
                        physical.cache_key,
                        raster.width,
                        raster.height,
                        raster.data,
                    );

                    glyphs.push(PositionedGlyph {
                        cache_key: physical.cache_key,
                        x: bounds.x + padding_x + physical.x as f32 / sf + raster.left as f32 / sf,
                        y: bounds.y + padding_y + run.line_y + physical.y as f32 / sf
                            - raster.top as f32 / sf,
                        width: raster.width as f32 / sf,
                        height: raster.height as f32 / sf,
                    });
                }
            }
        }

        if !glyphs.is_empty() {
            cx.commands().draw_glyphs(glyphs, color);
        }

        if focused
            && let Some(ref editable) = inner.editable {
                if let Some(cursor_rect) = editable.cursor_rect() {
                    let cursor_color = self
                        .style
                        .text_color
                        .unwrap_or(velox_scene::Color::rgb(0, 0, 0));
                    cx.commands().fill_rect(
                        Rect::new(
                            bounds.x + padding_x + cursor_rect.x,
                            bounds.y + padding_y + cursor_rect.y,
                            1.5,
                            cursor_rect.height,
                        ),
                        cursor_color,
                    );
                }

                for sel_rect in editable.selection_rects() {
                    cx.commands().fill_rect(
                        Rect::new(
                            bounds.x + padding_x + sel_rect.x,
                            bounds.y + padding_y + sel_rect.y,
                            sel_rect.width,
                            sel_rect.height,
                        ),
                        velox_scene::Color::rgba(0, 120, 215, 80),
                    );
                }
            }
    }
}

impl Input {
    fn paint_placeholder(&self, bounds: Rect, cx: &mut PaintContext) {
        let Some(ref placeholder) = self.placeholder else {
            return;
        };

        let placeholder_color = velox_scene::Color::rgb(160, 160, 160);
        let font_size = self.style.font_size.unwrap_or(14.0);
        let line_height = self.style.line_height.unwrap_or(font_size * 1.2);
        let padding_x = length_px(self.style.padding_left);

        let mut buffer = velox_text::TextBuffer::new(cx.font_system(), font_size, line_height);
        buffer.set_text(
            cx.font_system(),
            placeholder,
            velox_text::TextAttrs::default(),
        );
        buffer.shape(cx.font_system());

        let sf = cx.scale_factor();
        let text_height = buffer
            .layout_runs()
            .last()
            .map(|r| r.line_y + line_height)
            .unwrap_or(line_height);
        let y_center = bounds.y + (bounds.height - text_height) / 2.0;

        let mut glyphs = Vec::new();
        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), sf);
                let rasterized = cx
                    .glyph_rasterizer
                    .rasterize(cx.font_system, physical.cache_key);
                let Some(raster) = rasterized else {
                    continue;
                };
                if raster.width == 0 || raster.height == 0 {
                    continue;
                }
                cx.commands().upload_glyph(
                    physical.cache_key,
                    raster.width,
                    raster.height,
                    raster.data,
                );
                glyphs.push(PositionedGlyph {
                    cache_key: physical.cache_key,
                    x: bounds.x + padding_x + physical.x as f32 / sf + raster.left as f32 / sf,
                    y: y_center + run.line_y + physical.y as f32 / sf - raster.top as f32 / sf,
                    width: raster.width as f32 / sf,
                    height: raster.height as f32 / sf,
                });
            }
        }

        if !glyphs.is_empty() {
            cx.commands().draw_glyphs(glyphs, placeholder_color);
        }
    }

    fn paint_cursor_at_origin(&self, bounds: Rect, cx: &mut PaintContext) {
        let font_size = self.style.font_size.unwrap_or(14.0);
        let line_height = self.style.line_height.unwrap_or(font_size * 1.2);
        let padding_x = length_px(self.style.padding_left);
        let y_center = bounds.y + (bounds.height - line_height) / 2.0;
        let cursor_color = self
            .style
            .text_color
            .unwrap_or(velox_scene::Color::rgb(0, 0, 0));
        cx.commands().fill_rect(
            Rect::new(bounds.x + padding_x, y_center, 1.5, line_height),
            cursor_color,
        );
    }
}

fn accessibility_text_runs(
    editable: &velox_text::EditableText,
    style: &Style,
) -> Vec<velox_scene::AccessibilityTextRun> {
    let padding_x = length_px(style.padding_left);
    let padding_y = length_px(style.padding_top);

    editable
        .buffer()
        .accessibility_runs(editable.text())
        .into_iter()
        .map(|run| {
            velox_scene::AccessibilityTextRun::new(
                run.text,
                run.byte_start,
                Rect::new(run.x + padding_x, run.y + padding_y, run.width, run.height),
            )
        })
        .collect()
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
    fn input_handle_persists_text() {
        let handle = InputHandle::new();

        {
            let mut inner = handle.inner.borrow_mut();
            let mut fs = velox_text::FontSystem::new();
            let mut editable = velox_text::EditableText::new(&mut fs, 14.0, 16.8, false);
            editable.insert_char(&mut fs, 'H');
            editable.insert_char(&mut fs, 'i');
            inner.editable = Some(editable);
        }

        assert_eq!(handle.text(), "Hi");
    }

    #[test]
    fn input_handle_clone_shares_state() {
        let handle = InputHandle::new();
        let handle2 = handle.clone();

        {
            let mut inner = handle.inner.borrow_mut();
            inner.focused = true;
        }

        assert!(handle2.is_focused());
    }

    #[test]
    fn pending_events_queued_via_key_handler() {
        let handle = InputHandle::new();
        let mut inp = input().handle(handle.clone()).focusable();
        let handlers = inp.take_handlers();

        let key_handler = handlers.on_key_down.unwrap();
        key_handler(&velox_scene::KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: velox_scene::KeyState::Pressed,
            text: Some("a".to_string()),
        });

        let inner = handle.inner.borrow();
        assert_eq!(inner.pending.len(), 1);
        assert!(matches!(inner.pending[0], PendingInput::Text(ref s) if s == "a"));
    }

    #[test]
    fn focus_handler_updates_handle() {
        let handle = InputHandle::new();
        let mut inp = input().handle(handle.clone()).focusable();
        let handlers = inp.take_handlers();

        let focus_handler = handlers.on_focus.unwrap();
        focus_handler(true);
        assert!(handle.is_focused());

        focus_handler(false);
        assert!(!handle.is_focused());
    }

    #[test]
    fn on_change_callback_fires() {
        use std::cell::Cell;
        use std::rc::Rc;

        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        let handle = InputHandle::new();
        let mut inp = input()
            .handle(handle.clone())
            .on_change(move |_text| {
                c.set(true);
            })
            .focusable();
        let _handlers = inp.take_handlers();

        let inner = handle.inner.borrow();
        assert!(inner.on_change.is_some());
    }

    #[test]
    fn input_handle_value_actions_queue_pending_updates() {
        let handle = InputHandle::new();

        handle.set_text("Hello");
        handle.select_all();
        handle.replace_selected_text("World");
        handle.set_selection(velox_text::TextSelection {
            anchor: 1,
            focus: 3,
        });

        let inner = handle.inner.borrow();
        assert!(matches!(inner.pending[0], PendingInput::SetText(ref s) if s == "Hello"));
        assert!(matches!(inner.pending[1], PendingInput::SelectAll));
        assert!(
            matches!(inner.pending[2], PendingInput::ReplaceSelectedText(ref s) if s == "World")
        );
        assert!(matches!(
            inner.pending[3],
            PendingInput::SetSelection(velox_text::TextSelection {
                anchor: 1,
                focus: 3
            })
        ));
    }
}
