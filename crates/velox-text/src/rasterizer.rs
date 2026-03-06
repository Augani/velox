use cosmic_text::{CacheKey, SwashCache, SwashContent};

use crate::font_system::FontSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphContentType {
    Mask,
    Color,
}

pub struct RasterizedGlyph {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub data: Vec<u8>,
    pub content_type: GlyphContentType,
}

impl RasterizedGlyph {
    pub fn is_color(&self) -> bool {
        self.content_type == GlyphContentType::Color
    }
}

pub struct GlyphRasterizer {
    swash_cache: SwashCache,
}

impl GlyphRasterizer {
    pub fn new() -> Self {
        Self {
            swash_cache: SwashCache::new(),
        }
    }

    pub fn rasterize(
        &mut self,
        font_system: &mut FontSystem,
        cache_key: CacheKey,
    ) -> Option<RasterizedGlyph> {
        let image = self
            .swash_cache
            .get_image(font_system.inner_mut(), cache_key)
            .as_ref()?;
        let content_type = match image.content {
            SwashContent::Color => GlyphContentType::Color,
            _ => GlyphContentType::Mask,
        };
        Some(RasterizedGlyph {
            width: image.placement.width,
            height: image.placement.height,
            left: image.placement.left,
            top: image.placement.top,
            data: image.data.clone(),
            content_type,
        })
    }
}

impl Default for GlyphRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attrs::TextAttrs;
    use crate::buffer::TextBuffer;
    use crate::font_system::FontSystem;

    #[test]
    fn rasterize_glyphs_from_buffer() {
        let mut fs = FontSystem::new();
        let mut rasterizer = GlyphRasterizer::new();
        let mut buf = TextBuffer::new(&mut fs, 24.0, 30.0);
        buf.set_size(&mut fs, 400.0, 100.0);
        buf.set_text(&mut fs, "Hello", TextAttrs::default());
        buf.shape(&mut fs);

        let mut rasterized_count = 0;
        for run in buf.layout_runs() {
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0.0, 0.0), 1.0);
                if let Some(image) = rasterizer.rasterize(&mut fs, physical.cache_key) {
                    assert!(image.width > 0 || image.height > 0 || image.data.is_empty());
                    rasterized_count += 1;
                }
            }
        }
        assert!(rasterized_count > 0);
    }
}
