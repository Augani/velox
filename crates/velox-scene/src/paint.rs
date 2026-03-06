use crate::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub u64);

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

#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub offset: f32,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Gradient {
    Linear {
        angle_deg: f32,
        stops: Vec<GradientStop>,
    },
    Radial {
        center_x: f32,
        center_y: f32,
        stops: Vec<GradientStop>,
    },
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
    DrawImage {
        texture_id: TextureId,
        src_rect: Rect,
        dst_rect: Rect,
        opacity: f32,
    },
    PushClip(Rect),
    PopClip,
    PushLayer {
        opacity: f32,
        blend_mode: BlendMode,
    },
    PopLayer,
    BoxShadow {
        rect: Rect,
        color: Color,
        blur_radius: f32,
        offset: crate::geometry::Point,
        spread: f32,
    },
    FillGradient {
        rect: Rect,
        gradient: Gradient,
    },
}

pub struct GlyphUpload {
    pub cache_key: cosmic_text::CacheKey,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub struct CommandList {
    commands: Vec<PaintCommand>,
    glyph_uploads: Vec<GlyphUpload>,
    epoch: u64,
}

impl CommandList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            glyph_uploads: Vec::new(),
            epoch: 0,
        }
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.bump_epoch();
        self.commands.push(PaintCommand::FillRect { rect, color });
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.bump_epoch();
        self.commands
            .push(PaintCommand::StrokeRect { rect, color, width });
    }

    pub fn draw_glyphs(&mut self, glyphs: Vec<PositionedGlyph>, color: Color) {
        self.bump_epoch();
        self.commands
            .push(PaintCommand::DrawGlyphs { glyphs, color });
    }

    pub fn draw_image(
        &mut self,
        texture_id: TextureId,
        src_rect: Rect,
        dst_rect: Rect,
        opacity: f32,
    ) {
        self.bump_epoch();
        self.commands.push(PaintCommand::DrawImage {
            texture_id,
            src_rect,
            dst_rect,
            opacity,
        });
    }

    pub fn push_clip(&mut self, rect: Rect) {
        self.bump_epoch();
        self.commands.push(PaintCommand::PushClip(rect));
    }

    pub fn pop_clip(&mut self) {
        self.bump_epoch();
        self.commands.push(PaintCommand::PopClip);
    }

    pub fn push_layer(&mut self, opacity: f32, blend_mode: BlendMode) {
        self.bump_epoch();
        self.commands.push(PaintCommand::PushLayer {
            opacity,
            blend_mode,
        });
    }

    pub fn pop_layer(&mut self) {
        self.bump_epoch();
        self.commands.push(PaintCommand::PopLayer);
    }

    pub fn fill_gradient(&mut self, rect: Rect, gradient: Gradient) {
        self.bump_epoch();
        self.commands
            .push(PaintCommand::FillGradient { rect, gradient });
    }

    pub fn box_shadow(
        &mut self,
        rect: Rect,
        color: Color,
        blur_radius: f32,
        offset: crate::geometry::Point,
        spread: f32,
    ) {
        self.bump_epoch();
        self.commands.push(PaintCommand::BoxShadow {
            rect,
            color,
            blur_radius,
            offset,
            spread,
        });
    }

    pub fn upload_glyph(
        &mut self,
        cache_key: cosmic_text::CacheKey,
        width: u32,
        height: u32,
        data: Vec<u8>,
    ) {
        self.bump_epoch();
        self.glyph_uploads.push(GlyphUpload {
            cache_key,
            width,
            height,
            data,
        });
    }

    pub fn glyph_uploads(&self) -> &[GlyphUpload] {
        &self.glyph_uploads
    }

    pub fn commands(&self) -> &[PaintCommand] {
        &self.commands
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn clear(&mut self) {
        self.bump_epoch();
        self.commands.clear();
        self.glyph_uploads.clear();
    }

    fn bump_epoch(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
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

    #[test]
    fn texture_id_equality() {
        let a = TextureId(1);
        let b = TextureId(1);
        let c = TextureId(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn push_pop_layer_commands() {
        let mut list = CommandList::new();
        list.push_layer(0.5, BlendMode::Normal);
        list.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::rgb(255, 0, 0));
        list.pop_layer();
        assert_eq!(list.commands().len(), 3);
        assert!(matches!(list.commands()[0], PaintCommand::PushLayer { .. }));
        assert!(matches!(list.commands()[2], PaintCommand::PopLayer));
    }

    #[test]
    fn box_shadow_command() {
        use crate::geometry::Point;
        let mut list = CommandList::new();
        list.box_shadow(
            Rect::new(10.0, 10.0, 100.0, 50.0),
            Color::rgba(0, 0, 0, 128),
            8.0,
            Point::new(2.0, 4.0),
            0.0,
        );
        assert_eq!(list.commands().len(), 1);
        assert!(matches!(list.commands()[0], PaintCommand::BoxShadow { .. }));
    }

    #[test]
    fn draw_image_command() {
        let mut list = CommandList::new();
        let tid = TextureId(42);
        let src = Rect::new(0.0, 0.0, 64.0, 64.0);
        let dst = Rect::new(10.0, 20.0, 128.0, 128.0);
        list.draw_image(tid, src, dst, 0.8);

        assert_eq!(list.commands().len(), 1);
        match &list.commands()[0] {
            PaintCommand::DrawImage {
                texture_id,
                src_rect,
                dst_rect,
                opacity,
            } => {
                assert_eq!(*texture_id, tid);
                assert_eq!(*src_rect, src);
                assert_eq!(*dst_rect, dst);
                assert_eq!(*opacity, 0.8);
            }
            _ => panic!("expected DrawImage"),
        }
    }
}
