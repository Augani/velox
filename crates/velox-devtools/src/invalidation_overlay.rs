use velox_scene::{Color, CommandList, Rect};

const OVERLAY_COLOR: Color = Color {
    r: 255,
    g: 50,
    b: 50,
    a: 40,
};
const BORDER_COLOR: Color = Color {
    r: 255,
    g: 50,
    b: 50,
    a: 120,
};
const BORDER_WIDTH: f32 = 1.0;
const MAX_REGIONS: usize = 64;

pub struct InvalidationOverlay {
    regions: Vec<Rect>,
    enabled: bool,
}

impl InvalidationOverlay {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            enabled: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.regions.clear();
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn record_regions(&mut self, regions: &[Rect]) {
        if !self.enabled {
            return;
        }
        self.regions.clear();
        let take = regions.len().min(MAX_REGIONS);
        self.regions.extend_from_slice(&regions[..take]);
    }

    pub fn paint(&self, commands: &mut CommandList) {
        if !self.enabled {
            return;
        }
        for rect in &self.regions {
            commands.fill_rect(*rect, OVERLAY_COLOR);
            commands.stroke_rect(*rect, BORDER_COLOR, BORDER_WIDTH);
        }
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn regions(&self) -> &[Rect] {
        &self.regions
    }
}

impl Default for InvalidationOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        let overlay = InvalidationOverlay::new();
        assert!(!overlay.is_enabled());
        assert_eq!(overlay.region_count(), 0);
    }

    #[test]
    fn records_regions_when_enabled() {
        let mut overlay = InvalidationOverlay::new();
        overlay.set_enabled(true);
        overlay.record_regions(&[
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(20.0, 20.0, 30.0, 30.0),
        ]);
        assert_eq!(overlay.region_count(), 2);
    }

    #[test]
    fn ignores_regions_when_disabled() {
        let mut overlay = InvalidationOverlay::new();
        overlay.record_regions(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        assert_eq!(overlay.region_count(), 0);
    }

    #[test]
    fn disable_clears_regions() {
        let mut overlay = InvalidationOverlay::new();
        overlay.set_enabled(true);
        overlay.record_regions(&[Rect::new(0.0, 0.0, 10.0, 10.0)]);
        overlay.set_enabled(false);
        assert_eq!(overlay.region_count(), 0);
    }

    #[test]
    fn paint_emits_commands_when_enabled() {
        let mut overlay = InvalidationOverlay::new();
        overlay.set_enabled(true);
        overlay.record_regions(&[Rect::new(5.0, 5.0, 50.0, 50.0)]);

        let mut commands = CommandList::new();
        overlay.paint(&mut commands);
        assert_eq!(commands.commands().len(), 2);
    }

    #[test]
    fn paint_emits_nothing_when_disabled() {
        let overlay = InvalidationOverlay::new();
        let mut commands = CommandList::new();
        overlay.paint(&mut commands);
        assert!(commands.commands().is_empty());
    }

    #[test]
    fn bounds_regions_to_max() {
        let mut overlay = InvalidationOverlay::new();
        overlay.set_enabled(true);
        let many: Vec<Rect> = (0..100)
            .map(|i| Rect::new(i as f32, 0.0, 1.0, 1.0))
            .collect();
        overlay.record_regions(&many);
        assert!(overlay.region_count() <= 64);
    }
}
