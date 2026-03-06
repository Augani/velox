pub struct RenderStats {
    glyph_bytes_uploaded: u64,
    texture_bytes_uploaded: u64,
    frame_count: u64,
}

impl RenderStats {
    pub fn new() -> Self {
        Self {
            glyph_bytes_uploaded: 0,
            texture_bytes_uploaded: 0,
            frame_count: 0,
        }
    }

    pub fn record_glyph_upload(&mut self, bytes: u64) {
        self.glyph_bytes_uploaded = self.glyph_bytes_uploaded.saturating_add(bytes);
    }

    pub fn record_texture_upload(&mut self, bytes: u64) {
        self.texture_bytes_uploaded = self.texture_bytes_uploaded.saturating_add(bytes);
    }

    pub fn tick_frame(&mut self) {
        self.frame_count = self.frame_count.saturating_add(1);
    }

    pub fn total_bytes_uploaded(&self) -> u64 {
        self.glyph_bytes_uploaded
            .saturating_add(self.texture_bytes_uploaded)
    }

    pub fn glyph_bytes_uploaded(&self) -> u64 {
        self.glyph_bytes_uploaded
    }

    pub fn texture_bytes_uploaded(&self) -> u64 {
        self.texture_bytes_uploaded
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn reset(&mut self) {
        self.glyph_bytes_uploaded = 0;
        self.texture_bytes_uploaded = 0;
        self.frame_count = 0;
    }
}

impl Default for RenderStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_zero() {
        let stats = RenderStats::new();
        assert_eq!(stats.total_bytes_uploaded(), 0);
        assert_eq!(stats.frame_count(), 0);
    }

    #[test]
    fn tracks_glyph_uploads() {
        let mut stats = RenderStats::new();
        stats.record_glyph_upload(1024);
        stats.record_glyph_upload(512);
        assert_eq!(stats.glyph_bytes_uploaded(), 1536);
        assert_eq!(stats.total_bytes_uploaded(), 1536);
    }

    #[test]
    fn tracks_texture_uploads() {
        let mut stats = RenderStats::new();
        stats.record_texture_upload(2048);
        assert_eq!(stats.texture_bytes_uploaded(), 2048);
        assert_eq!(stats.total_bytes_uploaded(), 2048);
    }

    #[test]
    fn total_combines_both() {
        let mut stats = RenderStats::new();
        stats.record_glyph_upload(100);
        stats.record_texture_upload(200);
        assert_eq!(stats.total_bytes_uploaded(), 300);
    }

    #[test]
    fn tick_frame_increments() {
        let mut stats = RenderStats::new();
        stats.tick_frame();
        stats.tick_frame();
        stats.tick_frame();
        assert_eq!(stats.frame_count(), 3);
    }

    #[test]
    fn reset_clears_all() {
        let mut stats = RenderStats::new();
        stats.record_glyph_upload(100);
        stats.record_texture_upload(200);
        stats.tick_frame();
        stats.reset();
        assert_eq!(stats.total_bytes_uploaded(), 0);
        assert_eq!(stats.frame_count(), 0);
    }
}
