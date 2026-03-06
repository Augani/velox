use crate::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub cache_key: cosmic_text::CacheKey,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub enum PaintCommand {
    FillRect {
        rect: Rect,
        color: Color,
    },
    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
    },
    DrawGlyphs {
        glyphs: Vec<PositionedGlyph>,
        color: Color,
    },
    PushClip(Rect),
    PopClip,
}

pub struct CommandList {
    commands: Vec<PaintCommand>,
}

impl CommandList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(PaintCommand::FillRect { rect, color });
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.commands
            .push(PaintCommand::StrokeRect { rect, color, width });
    }

    pub fn draw_glyphs(&mut self, glyphs: Vec<PositionedGlyph>, color: Color) {
        self.commands
            .push(PaintCommand::DrawGlyphs { glyphs, color });
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.commands.push(PaintCommand::PushClip(rect));
    }

    pub fn pop_clip(&mut self) {
        self.commands.push(PaintCommand::PopClip);
    }

    pub fn commands(&self) -> &[PaintCommand] {
        &self.commands
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

impl Default for CommandList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_list() {
        let list = CommandList::new();
        assert!(list.commands().is_empty());
    }

    #[test]
    fn push_fill_rect() {
        let mut list = CommandList::new();
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let color = Color::rgb(255, 0, 0);
        list.fill_rect(rect, color);

        assert_eq!(list.commands().len(), 1);
        match &list.commands()[0] {
            PaintCommand::FillRect { rect: r, color: c } => {
                assert_eq!(*r, rect);
                assert_eq!(*c, color);
            }
            _ => panic!("expected FillRect"),
        }
    }

    #[test]
    fn push_clip_and_pop() {
        let mut list = CommandList::new();
        let clip_rect = Rect::new(0.0, 0.0, 200.0, 200.0);
        let fill_rect = Rect::new(10.0, 10.0, 50.0, 50.0);
        let color = Color::rgb(0, 255, 0);

        list.push_clip(clip_rect);
        list.fill_rect(fill_rect, color);
        list.pop_clip();

        assert_eq!(list.commands().len(), 3);
        assert!(matches!(list.commands()[0], PaintCommand::PushClip(_)));
        assert!(matches!(list.commands()[1], PaintCommand::FillRect { .. }));
        assert!(matches!(list.commands()[2], PaintCommand::PopClip));
    }

    #[test]
    fn clear_resets_list() {
        let mut list = CommandList::new();
        list.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::rgb(0, 0, 0));
        assert_eq!(list.commands().len(), 1);

        list.clear();
        assert!(list.commands().is_empty());
    }

    #[test]
    fn stroke_rect() {
        let mut list = CommandList::new();
        let rect = Rect::new(5.0, 5.0, 80.0, 60.0);
        let color = Color::rgba(0, 0, 255, 128);
        list.stroke_rect(rect, color, 2.0);

        assert_eq!(list.commands().len(), 1);
        match &list.commands()[0] {
            PaintCommand::StrokeRect {
                rect: r,
                color: c,
                width,
            } => {
                assert_eq!(*r, rect);
                assert_eq!(*c, color);
                assert_eq!(*width, 2.0);
            }
            _ => panic!("expected StrokeRect"),
        }
    }
}
