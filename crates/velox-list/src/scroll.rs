#[derive(Debug, Clone, Copy)]
pub struct ScrollAnchor {
    pub index: usize,
    pub offset: f32,
}

#[derive(Debug, Clone)]
pub struct ScrollState {
    pub offset: f32,
    pub content_height: f32,
    pub viewport_height: f32,
}

impl ScrollState {
    pub fn max_offset(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    pub fn clamp_offset(&mut self) {
        self.offset = self.offset.clamp(0.0, self.max_offset());
    }

    pub fn scroll_by(&mut self, delta: f32) {
        self.offset += delta;
        self.clamp_offset();
    }

    pub fn scroll_fraction(&self) -> f32 {
        let max = self.max_offset();
        if max <= 0.0 {
            return 0.0;
        }
        self.offset / max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_offset_when_content_smaller_than_viewport() {
        let state = ScrollState {
            offset: 0.0,
            content_height: 50.0,
            viewport_height: 100.0,
        };
        assert_eq!(state.max_offset(), 0.0);
    }

    #[test]
    fn clamp_offset_stays_in_bounds() {
        let mut state = ScrollState {
            offset: 500.0,
            content_height: 300.0,
            viewport_height: 100.0,
        };
        state.clamp_offset();
        assert_eq!(state.offset, 200.0);

        state.offset = -10.0;
        state.clamp_offset();
        assert_eq!(state.offset, 0.0);
    }

    #[test]
    fn scroll_by_clamps() {
        let mut state = ScrollState {
            offset: 0.0,
            content_height: 200.0,
            viewport_height: 100.0,
        };
        state.scroll_by(50.0);
        assert_eq!(state.offset, 50.0);

        state.scroll_by(200.0);
        assert_eq!(state.offset, 100.0);

        state.scroll_by(-300.0);
        assert_eq!(state.offset, 0.0);
    }

    #[test]
    fn scroll_fraction_edge_cases() {
        let state = ScrollState {
            offset: 0.0,
            content_height: 100.0,
            viewport_height: 100.0,
        };
        assert_eq!(state.scroll_fraction(), 0.0);

        let state = ScrollState {
            offset: 50.0,
            content_height: 200.0,
            viewport_height: 100.0,
        };
        assert_eq!(state.scroll_fraction(), 0.5);
    }
}
