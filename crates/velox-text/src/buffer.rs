use cosmic_text::{Buffer, Metrics, Shaping};

use crate::attrs::TextAttrs;
use crate::font_system::FontSystem;

pub struct TextBuffer {
    inner: Buffer,
}

impl TextBuffer {
    pub fn new(font_system: &mut FontSystem, font_size: f32, line_height: f32) -> Self {
        Self {
            inner: Buffer::new(
                font_system.inner_mut(),
                Metrics::new(font_size, line_height),
            ),
        }
    }

    pub fn set_size(&mut self, font_system: &mut FontSystem, width: f32, height: f32) {
        self.inner
            .set_size(font_system.inner_mut(), Some(width), Some(height));
    }

    pub fn set_text(&mut self, font_system: &mut FontSystem, text: &str, attrs: TextAttrs) {
        let cosmic_attrs = attrs.to_cosmic();
        self.inner.set_text(
            font_system.inner_mut(),
            text,
            cosmic_attrs,
            Shaping::Advanced,
        );
    }

    pub fn shape(&mut self, font_system: &mut FontSystem) {
        self.inner.shape_until_scroll(font_system.inner_mut(), true);
    }

    pub fn layout_runs(&self) -> impl Iterator<Item = cosmic_text::LayoutRun<'_>> {
        self.inner.layout_runs()
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &Buffer {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_system::FontSystem;

    #[test]
    fn create_buffer_and_shape() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, "Hello, world!", TextAttrs::default());
        buf.shape(&mut fs);
        let runs: Vec<_> = buf.layout_runs().collect();
        assert!(!runs.is_empty());
    }

    #[test]
    fn empty_text_produces_single_empty_run() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, "", TextAttrs::default());
        buf.shape(&mut fs);
        let runs: Vec<_> = buf.layout_runs().collect();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].glyphs.is_empty());
    }

    #[test]
    fn multiline_text() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, "Line 1\nLine 2\nLine 3", TextAttrs::default());
        buf.shape(&mut fs);
        let runs: Vec<_> = buf.layout_runs().collect();
        assert!(runs.len() >= 3);
    }
}
