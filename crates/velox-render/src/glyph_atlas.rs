use std::collections::HashMap;

use velox_text::cosmic_text::CacheKey;

#[derive(Debug, Clone, Copy)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

struct Shelf {
    y: u32,
    height: u32,
    x_cursor: u32,
}

pub(crate) struct ShelfPacker {
    width: u32,
    height: u32,
    shelves: Vec<Shelf>,
    y_cursor: u32,
}

impl ShelfPacker {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            shelves: Vec::new(),
            y_cursor: 0,
        }
    }

    pub fn allocate(&mut self, w: u32, h: u32) -> Option<AtlasRegion> {
        for shelf in &mut self.shelves {
            if h <= shelf.height && shelf.x_cursor + w <= self.width {
                let region = AtlasRegion {
                    x: shelf.x_cursor,
                    y: shelf.y,
                    width: w,
                    height: h,
                };
                shelf.x_cursor += w;
                return Some(region);
            }
        }

        if self.y_cursor + h > self.height {
            return None;
        }

        let region = AtlasRegion {
            x: 0,
            y: self.y_cursor,
            width: w,
            height: h,
        };
        self.shelves.push(Shelf {
            y: self.y_cursor,
            height: h,
            x_cursor: w,
        });
        self.y_cursor += h;
        Some(region)
    }
}

pub struct GlyphAtlas {
    packer: ShelfPacker,
    entries: HashMap<CacheKey, AtlasRegion>,
    texture_data: Vec<u8>,
    atlas_width: u32,
    atlas_height: u32,
    dirty: bool,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            packer: ShelfPacker::new(width, height),
            entries: HashMap::new(),
            texture_data: vec![0u8; (width * height) as usize],
            atlas_width: width,
            atlas_height: height,
            dirty: false,
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<&AtlasRegion> {
        self.entries.get(key)
    }

    pub fn insert(&mut self, key: CacheKey, w: u32, h: u32, data: &[u8]) -> Option<AtlasRegion> {
        if let Some(existing) = self.entries.get(&key) {
            return Some(*existing);
        }

        let region = self.packer.allocate(w, h)?;

        for row in 0..h {
            let src_start = (row * w) as usize;
            let src_end = src_start + w as usize;
            let dst_start = ((region.y + row) * self.atlas_width + region.x) as usize;
            let dst_end = dst_start + w as usize;
            if src_end <= data.len() && dst_end <= self.texture_data.len() {
                self.texture_data[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            }
        }

        self.entries.insert(key, region);
        self.dirty = true;
        Some(region)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn texture_data(&self) -> &[u8] {
        &self.texture_data
    }

    pub fn width(&self) -> u32 {
        self.atlas_width
    }

    pub fn height(&self) -> u32 {
        self.atlas_height
    }

    pub fn uv(&self, region: &AtlasRegion) -> [f32; 4] {
        let w = self.atlas_width as f32;
        let h = self.atlas_height as f32;
        [
            region.x as f32 / w,
            region.y as f32 / h,
            (region.x + region.width) as f32 / w,
            (region.y + region.height) as f32 / h,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_packer_insert_and_lookup() {
        let mut packer = ShelfPacker::new(256, 256);
        let region = packer.allocate(10, 12);
        assert!(region.is_some());
        let r = region.unwrap();
        assert_eq!(r.width, 10);
        assert_eq!(r.height, 12);
    }

    #[test]
    fn atlas_packer_fills_shelf() {
        let mut packer = ShelfPacker::new(64, 64);
        for _ in 0..6 {
            assert!(packer.allocate(10, 10).is_some());
        }
    }

    #[test]
    fn atlas_packer_returns_none_when_full() {
        let mut packer = ShelfPacker::new(16, 16);
        packer.allocate(16, 16);
        assert!(packer.allocate(1, 1).is_none());
    }
}
