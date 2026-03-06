use crate::attrs::TextAttrs;
use crate::buffer::TextBuffer;
use crate::composition::CompositionState;
use crate::font_system::FontSystem;
use crate::selection::TextSelection;
use crate::undo::{EditCommand, UndoStack};

#[derive(Debug, Clone, Copy)]
pub struct TextRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorDirection {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

pub struct EditableText {
    text: String,
    buffer: TextBuffer,
    selection: TextSelection,
    undo_stack: UndoStack,
    attrs: TextAttrs,
    composition: CompositionState,
    #[allow(dead_code)]
    multiline: bool,
}

impl EditableText {
    pub fn new(
        font_system: &mut FontSystem,
        font_size: f32,
        line_height: f32,
        multiline: bool,
    ) -> Self {
        Self {
            text: String::new(),
            buffer: TextBuffer::new(font_system, font_size, line_height),
            selection: TextSelection::default(),
            undo_stack: UndoStack::new(),
            attrs: TextAttrs::default(),
            composition: CompositionState::default(),
            multiline,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn selection(&self) -> TextSelection {
        self.selection
    }

    pub fn set_selection(&mut self, sel: TextSelection) {
        self.selection = sel;
    }

    pub fn set_size(&mut self, font_system: &mut FontSystem, width: f32, height: f32) {
        self.buffer.set_size(font_system, width, height);
    }

    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str) {
        self.text = text.to_owned();
        self.selection = TextSelection::collapsed(self.text.len());
        self.undo_stack.clear();
        self.reshape(font_system);
    }

    pub fn select_all(&mut self) {
        self.selection = TextSelection {
            anchor: 0,
            focus: self.text.len(),
        };
    }

    pub fn selected_text(&self) -> &str {
        self.selection.selected_text(&self.text)
    }

    pub fn insert_char(&mut self, font_system: &mut FontSystem, ch: char) {
        self.delete_selection_inner(font_system);
        let pos = self.selection.focus;
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.text.insert_str(pos, s);
        self.selection = TextSelection::collapsed(pos + s.len());
        self.undo_stack.push_coalesced(EditCommand::Insert {
            position: pos,
            text: s.to_owned(),
        });
        self.reshape(font_system);
    }

    pub fn insert_text(&mut self, font_system: &mut FontSystem, text: &str) {
        self.delete_selection_inner(font_system);
        let pos = self.selection.focus;
        self.text.insert_str(pos, text);
        self.selection = TextSelection::collapsed(pos + text.len());
        self.undo_stack.push(EditCommand::Insert {
            position: pos,
            text: text.to_owned(),
        });
        self.reshape(font_system);
    }

    pub fn delete_backward(&mut self, font_system: &mut FontSystem) {
        if !self.selection.is_collapsed() {
            self.delete_selection_inner(font_system);
            self.reshape(font_system);
            return;
        }
        let pos = self.selection.focus;
        if pos == 0 {
            return;
        }
        let prev = prev_char_boundary(&self.text, pos);
        let deleted = self.text[prev..pos].to_owned();
        self.text.replace_range(prev..pos, "");
        self.selection = TextSelection::collapsed(prev);
        self.undo_stack.push(EditCommand::Delete {
            position: prev,
            text: deleted,
        });
        self.reshape(font_system);
    }

    pub fn delete_forward(&mut self, font_system: &mut FontSystem) {
        if !self.selection.is_collapsed() {
            self.delete_selection_inner(font_system);
            self.reshape(font_system);
            return;
        }
        let pos = self.selection.focus;
        if pos >= self.text.len() {
            return;
        }
        let next = next_char_boundary(&self.text, pos);
        let deleted = self.text[pos..next].to_owned();
        self.text.replace_range(pos..next, "");
        self.undo_stack.push(EditCommand::Delete {
            position: pos,
            text: deleted,
        });
        self.reshape(font_system);
    }

    pub fn move_cursor_to(&mut self, _font_system: &mut FontSystem, index: usize) {
        let clamped = index.min(self.text.len());
        self.selection = TextSelection::collapsed(clamped);
    }

    pub fn move_cursor(
        &mut self,
        _font_system: &mut FontSystem,
        direction: CursorDirection,
        extend_selection: bool,
    ) {
        let pos = self.selection.focus;
        let new_pos = match direction {
            CursorDirection::Left => {
                if pos == 0 {
                    0
                } else {
                    prev_char_boundary(&self.text, pos)
                }
            }
            CursorDirection::Right => {
                if pos >= self.text.len() {
                    self.text.len()
                } else {
                    next_char_boundary(&self.text, pos)
                }
            }
            CursorDirection::Home => 0,
            CursorDirection::End => self.text.len(),
            CursorDirection::Up | CursorDirection::Down => pos,
        };

        if extend_selection {
            self.selection.focus = new_pos;
        } else {
            self.selection = TextSelection::collapsed(new_pos);
        }
    }

    pub fn undo(&mut self, font_system: &mut FontSystem) {
        let Some(cmd) = self.undo_stack.undo() else {
            return;
        };
        match &cmd {
            EditCommand::Insert { position, text } => {
                self.text
                    .replace_range(*position..*position + text.len(), "");
                self.selection = TextSelection::collapsed(*position);
            }
            EditCommand::Delete { position, text } => {
                self.text.insert_str(*position, text);
                self.selection = TextSelection::collapsed(*position + text.len());
            }
            EditCommand::Replace { position, old, new } => {
                self.text
                    .replace_range(*position..*position + new.len(), old);
                self.selection = TextSelection::collapsed(*position + old.len());
            }
        }
        self.reshape(font_system);
    }

    pub fn redo(&mut self, font_system: &mut FontSystem) {
        let Some(cmd) = self.undo_stack.redo() else {
            return;
        };
        match &cmd {
            EditCommand::Insert { position, text } => {
                self.text.insert_str(*position, text);
                self.selection = TextSelection::collapsed(*position + text.len());
            }
            EditCommand::Delete { position, text } => {
                self.text
                    .replace_range(*position..*position + text.len(), "");
                self.selection = TextSelection::collapsed(*position);
            }
            EditCommand::Replace { position, old, new } => {
                self.text
                    .replace_range(*position..*position + old.len(), new);
                self.selection = TextSelection::collapsed(*position + new.len());
            }
        }
        self.reshape(font_system);
    }

    pub fn hit_test(&self, _font_system: &FontSystem, x: f32, y: f32) -> usize {
        if x < 0.0 {
            return 0;
        }
        for run in self.buffer.layout_runs() {
            if y >= run.line_top && y < run.line_top + run.line_height {
                let mut last_end = 0;
                for glyph in run.glyphs.iter() {
                    let glyph_mid = glyph.x + glyph.w / 2.0;
                    if x < glyph_mid {
                        return glyph.start;
                    }
                    last_end = glyph.end;
                }
                return last_end;
            }
        }
        self.text.len()
    }

    pub fn cursor_rect(&self) -> Option<TextRect> {
        let pos = self.selection.focus;
        for run in self.buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                if pos >= glyph.start && pos <= glyph.end {
                    let x = if pos == glyph.start {
                        glyph.x
                    } else {
                        glyph.x + glyph.w
                    };
                    return Some(TextRect {
                        x,
                        y: run.line_top,
                        width: 1.5,
                        height: run.line_height,
                    });
                }
            }
            if run.glyphs.is_empty() || pos >= run.glyphs.last().map(|g| g.end).unwrap_or(0) {
                let x = run.glyphs.last().map(|g| g.x + g.w).unwrap_or(0.0);
                return Some(TextRect {
                    x,
                    y: run.line_top,
                    width: 1.5,
                    height: run.line_height,
                });
            }
        }
        Some(TextRect {
            x: 0.0,
            y: 0.0,
            width: 1.5,
            height: 20.0,
        })
    }

    pub fn selection_rects(&self) -> Vec<TextRect> {
        if self.selection.is_collapsed() {
            return Vec::new();
        }
        let (start, end) = self.selection.range();
        let mut rects = Vec::new();
        for run in self.buffer.layout_runs() {
            let mut line_start_x = None;
            let mut line_end_x = None;
            for glyph in run.glyphs.iter() {
                if glyph.end <= start || glyph.start >= end {
                    continue;
                }
                let gx_start = glyph.x;
                let gx_end = glyph.x + glyph.w;
                if line_start_x.is_none() {
                    line_start_x = Some(gx_start);
                }
                line_end_x = Some(gx_end);
            }
            if let (Some(sx), Some(ex)) = (line_start_x, line_end_x) {
                rects.push(TextRect {
                    x: sx,
                    y: run.line_top,
                    width: ex - sx,
                    height: run.line_height,
                });
            }
        }
        rects
    }

    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    pub fn composition(&self) -> &CompositionState {
        &self.composition
    }

    pub fn composition_mut(&mut self) -> &mut CompositionState {
        &mut self.composition
    }

    pub fn commit_composition(&mut self, font_system: &mut FontSystem) {
        if let Some(text) = self.composition.commit() {
            self.insert_text(font_system, &text);
        }
    }

    fn delete_selection_inner(&mut self, _font_system: &mut FontSystem) {
        if self.selection.is_collapsed() {
            return;
        }
        let (start, end) = self.selection.range();
        let deleted = self.text[start..end].to_owned();
        self.text.replace_range(start..end, "");
        self.selection = TextSelection::collapsed(start);
        self.undo_stack.push(EditCommand::Delete {
            position: start,
            text: deleted,
        });
    }

    fn reshape(&mut self, font_system: &mut FontSystem) {
        self.buffer
            .set_text(font_system, &self.text, self.attrs.clone());
        self.buffer.shape(font_system);
    }
}

fn prev_char_boundary(text: &str, index: usize) -> usize {
    let mut i = index.saturating_sub(1);
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    let mut i = index + 1;
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i.min(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_system::FontSystem;

    fn make_editable(fs: &mut FontSystem, text: &str) -> EditableText {
        let mut e = EditableText::new(fs, 14.0, 20.0, false);
        e.set_size(fs, 400.0, 100.0);
        if !text.is_empty() {
            e.set_text(fs, text);
        }
        e
    }

    #[test]
    fn insert_char() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "");
        e.insert_char(&mut fs, 'H');
        e.insert_char(&mut fs, 'i');
        assert_eq!(e.text(), "Hi");
        assert_eq!(e.selection().focus, 2);
    }

    #[test]
    fn delete_backward() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 5);
        e.delete_backward(&mut fs);
        assert_eq!(e.text(), "Hell");
    }

    #[test]
    fn delete_forward() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 0);
        e.delete_forward(&mut fs);
        assert_eq!(e.text(), "ello");
    }

    #[test]
    fn delete_selection() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello World");
        e.set_selection(TextSelection {
            anchor: 5,
            focus: 11,
        });
        e.delete_backward(&mut fs);
        assert_eq!(e.text(), "Hello");
    }

    #[test]
    fn insert_replaces_selection() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello World");
        e.set_selection(TextSelection {
            anchor: 0,
            focus: 5,
        });
        e.insert_char(&mut fs, 'Y');
        assert_eq!(e.text(), "Y World");
    }

    #[test]
    fn select_all() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.select_all();
        let sel = e.selection();
        assert_eq!(sel.anchor, 0);
        assert_eq!(sel.focus, 5);
    }

    #[test]
    fn insert_text_paste() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "AB");
        e.move_cursor_to(&mut fs, 1);
        e.insert_text(&mut fs, "xyz");
        assert_eq!(e.text(), "AxyzB");
        assert_eq!(e.selection().focus, 4);
    }

    #[test]
    fn undo_insert() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "");
        e.insert_char(&mut fs, 'A');
        e.insert_char(&mut fs, 'B');
        e.undo(&mut fs);
        assert_eq!(e.text(), "");
    }

    #[test]
    fn undo_then_redo() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "");
        e.insert_char(&mut fs, 'A');
        e.insert_char(&mut fs, 'B');
        e.undo(&mut fs);
        e.redo(&mut fs);
        assert_eq!(e.text(), "AB");
    }

    #[test]
    fn move_cursor_left_right() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 3);
        e.move_cursor(&mut fs, CursorDirection::Left, false);
        assert_eq!(e.selection().focus, 2);
        e.move_cursor(&mut fs, CursorDirection::Right, false);
        assert_eq!(e.selection().focus, 3);
    }

    #[test]
    fn move_cursor_with_shift_extends_selection() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 2);
        e.move_cursor(&mut fs, CursorDirection::Right, true);
        e.move_cursor(&mut fs, CursorDirection::Right, true);
        let sel = e.selection();
        assert_eq!(sel.anchor, 2);
        assert_eq!(sel.focus, 4);
    }

    #[test]
    fn home_end() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 3);
        e.move_cursor(&mut fs, CursorDirection::Home, false);
        assert_eq!(e.selection().focus, 0);
        e.move_cursor(&mut fs, CursorDirection::End, false);
        assert_eq!(e.selection().focus, 5);
    }

    #[test]
    fn cursor_clamps_at_boundaries() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hi");
        e.move_cursor_to(&mut fs, 0);
        e.move_cursor(&mut fs, CursorDirection::Left, false);
        assert_eq!(e.selection().focus, 0);
        e.move_cursor_to(&mut fs, 2);
        e.move_cursor(&mut fs, CursorDirection::Right, false);
        assert_eq!(e.selection().focus, 2);
    }

    #[test]
    fn hit_test_beginning() {
        let mut fs = FontSystem::new();
        let e = make_editable(&mut fs, "Hello World");
        let pos = e.hit_test(&fs, 0.0, 10.0);
        assert_eq!(pos, 0);
    }

    #[test]
    fn hit_test_end() {
        let mut fs = FontSystem::new();
        let e = make_editable(&mut fs, "Hello World");
        let pos = e.hit_test(&fs, 999.0, 10.0);
        assert_eq!(pos, 11);
    }

    #[test]
    fn hit_test_negative_returns_zero() {
        let mut fs = FontSystem::new();
        let e = make_editable(&mut fs, "Hello");
        let pos = e.hit_test(&fs, -10.0, 10.0);
        assert_eq!(pos, 0);
    }

    #[test]
    fn cursor_rect_at_start() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 0);
        let rect = e.cursor_rect();
        assert!(rect.is_some());
        let rect = rect.unwrap();
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
    }

    #[test]
    fn selection_rects_when_selected() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello World");
        e.set_selection(TextSelection {
            anchor: 0,
            focus: 5,
        });
        let rects = e.selection_rects();
        assert!(!rects.is_empty());
        assert!(rects[0].width > 0.0);
    }

    #[test]
    fn selection_rects_empty_when_collapsed() {
        let mut fs = FontSystem::new();
        let mut e = make_editable(&mut fs, "Hello");
        e.move_cursor_to(&mut fs, 2);
        let rects = e.selection_rects();
        assert!(rects.is_empty());
    }
}
