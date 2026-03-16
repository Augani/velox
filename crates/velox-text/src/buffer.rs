use cosmic_text::{Buffer, Metrics, Shaping};

use crate::attrs::TextAttrs;
use crate::font_system::FontSystem;

#[derive(Debug, Clone, PartialEq)]
pub struct TextRunLayout {
    pub text: String,
    pub byte_start: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

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

    pub fn set_metrics(&mut self, font_system: &mut FontSystem, font_size: f32, line_height: f32) {
        self.inner.set_metrics(
            font_system.inner_mut(),
            Metrics::new(font_size, line_height),
        );
    }

    pub fn layout_runs(&self) -> impl Iterator<Item = cosmic_text::LayoutRun<'_>> {
        self.inner.layout_runs()
    }

    pub fn accessibility_runs(&self, text: &str) -> Vec<TextRunLayout> {
        let mut runs = Vec::new();
        let mut current_line = None;
        let mut current_line_consumed = 0usize;
        let line_starts = line_start_offsets(text);

        for run in self.layout_runs() {
            if current_line != Some(run.line_i) {
                current_line = Some(run.line_i);
                current_line_consumed = 0;
            }
            let current_line_start = line_starts.get(run.line_i).copied().unwrap_or(text.len());

            let local_start = run
                .glyphs
                .first()
                .map(|glyph| glyph.start)
                .unwrap_or(current_line_consumed.min(run.text.len()));
            let local_end = run
                .glyphs
                .last()
                .map(|glyph| glyph.end)
                .unwrap_or(local_start);
            let byte_start = current_line_start + local_start;
            let x = run.glyphs.first().map(|glyph| glyph.x).unwrap_or(0.0);
            let width = match (run.glyphs.first(), run.glyphs.last()) {
                (Some(first), Some(last)) => (last.x + last.w - first.x).max(0.0),
                _ => 0.0,
            };

            let segment = run
                .text
                .get(local_start..local_end)
                .unwrap_or_default()
                .to_owned();

            runs.push(TextRunLayout {
                text: segment,
                byte_start,
                x,
                y: run.line_top,
                width,
                height: run.line_height,
            });

            current_line_consumed = current_line_consumed.max(local_end);
        }

        if runs.is_empty() {
            runs.push(TextRunLayout {
                text: String::new(),
                byte_start: 0,
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            });
        }

        runs
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &Buffer {
        &self.inner
    }
}

fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    let bytes = text.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                index += 2;
                starts.push(index);
            }
            b'\n' if bytes.get(index + 1) == Some(&b'\r') => {
                index += 2;
                starts.push(index);
            }
            b'\r' | b'\n' => {
                index += 1;
                starts.push(index);
            }
            _ => index += 1,
        }
    }

    starts
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

    #[test]
    fn accessibility_runs_capture_multiline_offsets() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        let text = "One\nTwo";
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, text, TextAttrs::default());
        buf.shape(&mut fs);

        let runs = buf.accessibility_runs(text);

        assert!(runs.len() >= 2);
        assert_eq!(runs[0].text, "One");
        assert_eq!(runs[0].byte_start, 0);
        assert_eq!(runs[1].text, "Two");
        assert_eq!(runs[1].byte_start, 4);
    }

    #[test]
    fn accessibility_runs_capture_crlf_offsets() {
        let mut fs = FontSystem::new();
        let mut buf = TextBuffer::new(&mut fs, 14.0, 20.0);
        let text = "One\r\nTwo";
        buf.set_size(&mut fs, 400.0, 300.0);
        buf.set_text(&mut fs, text, TextAttrs::default());
        buf.shape(&mut fs);

        let runs = buf.accessibility_runs(text);

        assert!(runs.len() >= 2);
        assert_eq!(runs[0].text, "One");
        assert_eq!(runs[0].byte_start, 0);
        assert_eq!(runs[1].text, "Two");
        assert_eq!(runs[1].byte_start, 5);
    }

    #[test]
    fn line_start_offsets_handle_mixed_line_endings() {
        assert_eq!(line_start_offsets("A\nB"), vec![0, 2]);
        assert_eq!(line_start_offsets("A\r\nB"), vec![0, 3]);
        assert_eq!(line_start_offsets("A\rB"), vec![0, 2]);
        assert_eq!(line_start_offsets("A\n\rB"), vec![0, 3]);
        assert_eq!(line_start_offsets("A\r\n"), vec![0, 3]);
    }
}
