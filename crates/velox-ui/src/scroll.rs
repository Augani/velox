const FRICTION: f32 = 0.95;
const SPRING_STIFFNESS: f32 = 300.0;
const SPRING_DAMPING: f32 = 35.0;
const OVERSCROLL_LIMIT: f32 = 100.0;
const VELOCITY_THRESHOLD: f32 = 0.5;
const SCROLLBAR_FADE_DELAY: f32 = 1.5;
const SCROLLBAR_FADE_SPEED: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug)]
pub struct ScrollState {
    offset_x: f32,
    offset_y: f32,
    velocity_x: f32,
    velocity_y: f32,
    content_width: f32,
    content_height: f32,
    viewport_width: f32,
    viewport_height: f32,
    axis: ScrollAxis,
    scrollbar_opacity: f32,
    scrollbar_idle_time: f32,
    animating: bool,
}

impl ScrollState {
    pub fn new(axis: ScrollAxis) -> Self {
        Self {
            offset_x: 0.0,
            offset_y: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            viewport_width: 0.0,
            viewport_height: 0.0,
            axis,
            scrollbar_opacity: 0.0,
            scrollbar_idle_time: 0.0,
            animating: false,
        }
    }

    pub fn offset(&self) -> (f32, f32) {
        (self.offset_x, self.offset_y)
    }

    pub fn offset_y(&self) -> f32 {
        self.offset_y
    }

    pub fn offset_x(&self) -> f32 {
        self.offset_x
    }

    pub fn velocity(&self) -> (f32, f32) {
        (self.velocity_x, self.velocity_y)
    }

    pub fn is_animating(&self) -> bool {
        self.animating
    }

    pub fn scrollbar_opacity(&self) -> f32 {
        self.scrollbar_opacity
    }

    pub fn set_content_size(&mut self, width: f32, height: f32) {
        self.content_width = width;
        self.content_height = height;
    }

    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    pub fn content_size(&self) -> (f32, f32) {
        (self.content_width, self.content_height)
    }

    pub fn viewport_size(&self) -> (f32, f32) {
        (self.viewport_width, self.viewport_height)
    }

    pub fn max_offset_x(&self) -> f32 {
        (self.content_width - self.viewport_width).max(0.0)
    }

    pub fn max_offset_y(&self) -> f32 {
        (self.content_height - self.viewport_height).max(0.0)
    }

    pub fn scroll_by(&mut self, dx: f32, dy: f32) {
        if matches!(self.axis, ScrollAxis::Horizontal | ScrollAxis::Both) {
            self.offset_x += dx;
            self.velocity_x = dx;
        }
        if matches!(self.axis, ScrollAxis::Vertical | ScrollAxis::Both) {
            self.offset_y += dy;
            self.velocity_y = dy;
        }
        self.show_scrollbar();
        self.animating = true;
    }

    pub fn scroll_to(&mut self, x: f32, y: f32, animated: bool) {
        let target_x = x.clamp(0.0, self.max_offset_x());
        let target_y = y.clamp(0.0, self.max_offset_y());

        if animated {
            self.velocity_x = (target_x - self.offset_x) * 10.0;
            self.velocity_y = (target_y - self.offset_y) * 10.0;
            self.animating = true;
        } else {
            self.offset_x = target_x;
            self.offset_y = target_y;
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
        }
        self.show_scrollbar();
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.animating {
            self.tick_scrollbar_fade(dt);
            return false;
        }

        let mut changed = false;

        if matches!(self.axis, ScrollAxis::Horizontal | ScrollAxis::Both) {
            changed |= self.tick_axis_x(dt);
        }
        if matches!(self.axis, ScrollAxis::Vertical | ScrollAxis::Both) {
            changed |= self.tick_axis_y(dt);
        }

        let still = self.velocity_x.abs() < VELOCITY_THRESHOLD
            && self.velocity_y.abs() < VELOCITY_THRESHOLD
            && self.overscroll_x() == 0.0
            && self.overscroll_y() == 0.0;

        if still {
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
            self.animating = false;
        }

        self.tick_scrollbar_fade(dt);
        changed || !still
    }

    fn tick_axis_x(&mut self, dt: f32) -> bool {
        let overscroll = self.overscroll_x();
        let old_offset = self.offset_x;

        if overscroll.abs() > 0.01 {
            let spring_force = -SPRING_STIFFNESS * overscroll - SPRING_DAMPING * self.velocity_x;
            self.velocity_x += spring_force * dt;
        } else {
            self.velocity_x *= FRICTION;
        }

        self.offset_x += self.velocity_x * dt;
        self.offset_x = self
            .offset_x
            .clamp(-OVERSCROLL_LIMIT, self.max_offset_x() + OVERSCROLL_LIMIT);

        (self.offset_x - old_offset).abs() > 0.01
    }

    fn tick_axis_y(&mut self, dt: f32) -> bool {
        let overscroll = self.overscroll_y();
        let old_offset = self.offset_y;

        if overscroll.abs() > 0.01 {
            let spring_force = -SPRING_STIFFNESS * overscroll - SPRING_DAMPING * self.velocity_y;
            self.velocity_y += spring_force * dt;
        } else {
            self.velocity_y *= FRICTION;
        }

        self.offset_y += self.velocity_y * dt;
        self.offset_y = self
            .offset_y
            .clamp(-OVERSCROLL_LIMIT, self.max_offset_y() + OVERSCROLL_LIMIT);

        (self.offset_y - old_offset).abs() > 0.01
    }

    fn overscroll_x(&self) -> f32 {
        let max = self.max_offset_x();
        if self.offset_x < 0.0 {
            self.offset_x
        } else if self.offset_x > max {
            self.offset_x - max
        } else {
            0.0
        }
    }

    fn overscroll_y(&self) -> f32 {
        let max = self.max_offset_y();
        if self.offset_y < 0.0 {
            self.offset_y
        } else if self.offset_y > max {
            self.offset_y - max
        } else {
            0.0
        }
    }

    fn show_scrollbar(&mut self) {
        self.scrollbar_opacity = 1.0;
        self.scrollbar_idle_time = 0.0;
    }

    fn tick_scrollbar_fade(&mut self, dt: f32) {
        if self.scrollbar_opacity <= 0.0 {
            return;
        }
        self.scrollbar_idle_time += dt;
        if self.scrollbar_idle_time > SCROLLBAR_FADE_DELAY {
            self.scrollbar_opacity = (self.scrollbar_opacity - SCROLLBAR_FADE_SPEED * dt).max(0.0);
        }
    }

    pub fn thumb_y_rect(&self, track_height: f32) -> Option<(f32, f32)> {
        if self.content_height <= self.viewport_height || self.content_height <= 0.0 {
            return None;
        }
        let ratio = self.viewport_height / self.content_height;
        let thumb_height = (ratio * track_height).max(20.0).min(track_height);
        let scrollable = self.max_offset_y();
        let clamped_offset = self.offset_y.clamp(0.0, scrollable);
        let progress = if scrollable > 0.0 {
            clamped_offset / scrollable
        } else {
            0.0
        };
        let thumb_y = progress * (track_height - thumb_height);
        Some((thumb_y, thumb_height))
    }

    pub fn thumb_x_rect(&self, track_width: f32) -> Option<(f32, f32)> {
        if self.content_width <= self.viewport_width || self.content_width <= 0.0 {
            return None;
        }
        let ratio = self.viewport_width / self.content_width;
        let thumb_width = (ratio * track_width).max(20.0).min(track_width);
        let scrollable = self.max_offset_x();
        let clamped_offset = self.offset_x.clamp(0.0, scrollable);
        let progress = if scrollable > 0.0 {
            clamped_offset / scrollable
        } else {
            0.0
        };
        let thumb_x = progress * (track_width - thumb_width);
        Some((thumb_x, thumb_width))
    }
}

const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLLBAR_PADDING: f32 = 2.0;
const SCROLLBAR_RADIUS: f32 = 4.0;

pub struct ScrollbarColors {
    pub track: velox_scene::Color,
    pub thumb: velox_scene::Color,
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        Self {
            track: velox_scene::Color::rgba(0, 0, 0, 20),
            thumb: velox_scene::Color::rgba(0, 0, 0, 100),
        }
    }
}

impl ScrollState {
    pub fn paint_scrollbars(
        &self,
        commands: &mut velox_scene::CommandList,
        bounds: velox_scene::Rect,
        colors: &ScrollbarColors,
    ) {
        if self.scrollbar_opacity <= 0.0 {
            return;
        }

        let use_layer = self.scrollbar_opacity < 1.0;
        if use_layer {
            commands.push_layer(self.scrollbar_opacity, velox_scene::BlendMode::Normal);
        }

        if matches!(self.axis, ScrollAxis::Vertical | ScrollAxis::Both) {
            self.paint_vertical_scrollbar(commands, bounds, colors);
        }
        if matches!(self.axis, ScrollAxis::Horizontal | ScrollAxis::Both) {
            self.paint_horizontal_scrollbar(commands, bounds, colors);
        }

        if use_layer {
            commands.pop_layer();
        }
    }

    fn paint_vertical_scrollbar(
        &self,
        commands: &mut velox_scene::CommandList,
        bounds: velox_scene::Rect,
        colors: &ScrollbarColors,
    ) {
        let track_x = bounds.x + bounds.width - SCROLLBAR_WIDTH - SCROLLBAR_PADDING;
        let track_y = bounds.y + SCROLLBAR_PADDING;
        let track_height = bounds.height - SCROLLBAR_PADDING * 2.0;

        let track_rect = velox_scene::Rect::new(track_x, track_y, SCROLLBAR_WIDTH, track_height);
        commands.fill_rounded_rect(track_rect, colors.track, SCROLLBAR_RADIUS);

        if let Some((thumb_offset, thumb_height)) = self.thumb_y_rect(track_height) {
            let thumb_rect = velox_scene::Rect::new(
                track_x,
                track_y + thumb_offset,
                SCROLLBAR_WIDTH,
                thumb_height,
            );
            commands.fill_rounded_rect(thumb_rect, colors.thumb, SCROLLBAR_RADIUS);
        }
    }

    fn paint_horizontal_scrollbar(
        &self,
        commands: &mut velox_scene::CommandList,
        bounds: velox_scene::Rect,
        colors: &ScrollbarColors,
    ) {
        let track_x = bounds.x + SCROLLBAR_PADDING;
        let track_y = bounds.y + bounds.height - SCROLLBAR_WIDTH - SCROLLBAR_PADDING;
        let track_width = bounds.width - SCROLLBAR_PADDING * 2.0;

        let track_rect = velox_scene::Rect::new(track_x, track_y, track_width, SCROLLBAR_WIDTH);
        commands.fill_rounded_rect(track_rect, colors.track, SCROLLBAR_RADIUS);

        if let Some((thumb_offset, thumb_width)) = self.thumb_x_rect(track_width) {
            let thumb_rect = velox_scene::Rect::new(
                track_x + thumb_offset,
                track_y,
                thumb_width,
                SCROLLBAR_WIDTH,
            );
            commands.fill_rounded_rect(thumb_rect, colors.thumb, SCROLLBAR_RADIUS);
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new(ScrollAxis::Vertical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_scroll() -> ScrollState {
        let mut state = ScrollState::new(ScrollAxis::Vertical);
        state.set_viewport_size(400.0, 600.0);
        state.set_content_size(400.0, 2000.0);
        state
    }

    #[test]
    fn initial_offset_is_zero() {
        let state = setup_scroll();
        assert_eq!(state.offset(), (0.0, 0.0));
        assert!(!state.is_animating());
    }

    #[test]
    fn scroll_by_changes_offset() {
        let mut state = setup_scroll();
        state.scroll_by(0.0, 100.0);
        assert_eq!(state.offset_y(), 100.0);
        assert!(state.is_animating());
    }

    #[test]
    fn scroll_to_clamps_to_max() {
        let mut state = setup_scroll();
        state.scroll_to(0.0, 5000.0, false);
        assert_eq!(state.offset_y(), state.max_offset_y());
    }

    #[test]
    fn max_offset_y_computed_correctly() {
        let state = setup_scroll();
        assert_eq!(state.max_offset_y(), 1400.0);
    }

    #[test]
    fn momentum_decays_with_friction() {
        let mut state = setup_scroll();
        state.scroll_by(0.0, 200.0);
        let initial_velocity = state.velocity().1;

        for _ in 0..60 {
            state.tick(1.0 / 60.0);
        }

        let final_velocity = state.velocity().1;
        assert!(
            final_velocity.abs() < initial_velocity.abs(),
            "velocity should decay: initial={}, final={}",
            initial_velocity,
            final_velocity,
        );
    }

    #[test]
    fn overscroll_springs_back() {
        let mut state = setup_scroll();
        state.offset_y = -50.0;
        state.animating = true;

        for _ in 0..300 {
            state.tick(1.0 / 60.0);
        }

        assert!(
            state.offset_y().abs() < 2.0,
            "should spring back near 0, got {}",
            state.offset_y(),
        );
    }

    #[test]
    fn overscroll_bottom_springs_back() {
        let mut state = setup_scroll();
        let max = state.max_offset_y();
        state.offset_y = max + 50.0;
        state.animating = true;

        for _ in 0..300 {
            state.tick(1.0 / 60.0);
        }

        assert!(
            (state.offset_y() - max).abs() < 2.0,
            "should spring back near max {}, got {}",
            max,
            state.offset_y(),
        );
    }

    #[test]
    fn axis_lock_vertical_ignores_horizontal() {
        let mut state = ScrollState::new(ScrollAxis::Vertical);
        state.set_viewport_size(400.0, 600.0);
        state.set_content_size(800.0, 2000.0);
        state.scroll_by(100.0, 50.0);
        assert_eq!(state.offset_x(), 0.0);
        assert_eq!(state.offset_y(), 50.0);
    }

    #[test]
    fn axis_lock_horizontal_ignores_vertical() {
        let mut state = ScrollState::new(ScrollAxis::Horizontal);
        state.set_viewport_size(400.0, 600.0);
        state.set_content_size(800.0, 2000.0);
        state.scroll_by(100.0, 50.0);
        assert_eq!(state.offset_x(), 100.0);
        assert_eq!(state.offset_y(), 0.0);
    }

    #[test]
    fn both_axes() {
        let mut state = ScrollState::new(ScrollAxis::Both);
        state.set_viewport_size(400.0, 600.0);
        state.set_content_size(800.0, 2000.0);
        state.scroll_by(50.0, 100.0);
        assert_eq!(state.offset_x(), 50.0);
        assert_eq!(state.offset_y(), 100.0);
    }

    #[test]
    fn scrollbar_shows_on_scroll_then_fades() {
        let mut state = setup_scroll();
        state.scroll_by(0.0, 10.0);
        assert_eq!(state.scrollbar_opacity(), 1.0);

        for _ in 0..90 {
            state.tick(1.0 / 60.0);
        }
        assert_eq!(state.scrollbar_opacity(), 1.0);

        for _ in 0..180 {
            state.tick(1.0 / 60.0);
        }
        assert!(
            state.scrollbar_opacity() < 1.0,
            "scrollbar should be fading, got {}",
            state.scrollbar_opacity(),
        );
    }

    #[test]
    fn thumb_y_proportional_to_viewport() {
        let state = setup_scroll();
        let (thumb_y, thumb_height) = state.thumb_y_rect(600.0).unwrap();
        assert_eq!(thumb_y, 0.0);
        let expected_height = (600.0 / 2000.0) * 600.0;
        assert!((thumb_height - expected_height).abs() < 0.01);
    }

    #[test]
    fn thumb_y_moves_with_scroll() {
        let mut state = setup_scroll();
        state.scroll_to(0.0, state.max_offset_y(), false);
        let (thumb_y, thumb_height) = state.thumb_y_rect(600.0).unwrap();
        let expected_bottom = 600.0 - thumb_height;
        assert!(
            (thumb_y - expected_bottom).abs() < 0.01,
            "thumb should be at bottom: expected {}, got {}",
            expected_bottom,
            thumb_y,
        );
    }

    #[test]
    fn thumb_none_when_content_fits() {
        let mut state = ScrollState::new(ScrollAxis::Vertical);
        state.set_viewport_size(400.0, 600.0);
        state.set_content_size(400.0, 500.0);
        assert!(state.thumb_y_rect(600.0).is_none());
    }

    #[test]
    fn animation_stops_when_settled() {
        let mut state = setup_scroll();
        state.scroll_by(0.0, 10.0);

        let mut ticks = 0;
        while state.is_animating() && ticks < 600 {
            state.tick(1.0 / 60.0);
            ticks += 1;
        }

        assert!(
            !state.is_animating(),
            "should stop animating after settling"
        );
        assert!(ticks < 600, "should settle within 10 seconds");
    }

    #[test]
    fn paint_scrollbar_no_commands_when_hidden() {
        let state = setup_scroll();
        let mut commands = velox_scene::CommandList::new();
        let bounds = velox_scene::Rect::new(0.0, 0.0, 400.0, 600.0);
        state.paint_scrollbars(&mut commands, bounds, &ScrollbarColors::default());
        assert!(
            commands.commands().is_empty(),
            "should not paint when opacity is 0"
        );
    }

    #[test]
    fn paint_scrollbar_emits_commands_when_visible() {
        let mut state = setup_scroll();
        state.scroll_by(0.0, 50.0);
        let mut commands = velox_scene::CommandList::new();
        let bounds = velox_scene::Rect::new(0.0, 0.0, 400.0, 600.0);
        state.paint_scrollbars(&mut commands, bounds, &ScrollbarColors::default());
        assert!(
            commands.commands().len() >= 2,
            "should emit track + thumb commands, got {}",
            commands.commands().len(),
        );
    }

    #[test]
    fn paint_scrollbar_uses_layer_when_fading() {
        let mut state = setup_scroll();
        state.scroll_by(0.0, 50.0);
        for _ in 0..95 {
            state.tick(1.0 / 60.0);
        }
        assert!(
            state.scrollbar_opacity() < 1.0,
            "opacity should have started fading, got {}",
            state.scrollbar_opacity(),
        );
        assert!(
            state.scrollbar_opacity() > 0.0,
            "opacity should not be fully faded yet, got {}",
            state.scrollbar_opacity(),
        );

        let mut commands = velox_scene::CommandList::new();
        let bounds = velox_scene::Rect::new(0.0, 0.0, 400.0, 600.0);
        state.paint_scrollbars(&mut commands, bounds, &ScrollbarColors::default());

        let cmds = commands.commands();
        assert!(
            matches!(
                cmds.first(),
                Some(velox_scene::PaintCommand::PushLayer { .. })
            ),
            "should push opacity layer when fading"
        );
        assert!(
            matches!(cmds.last(), Some(velox_scene::PaintCommand::PopLayer)),
            "should pop layer"
        );
    }

    #[test]
    fn paint_horizontal_scrollbar() {
        let mut state = ScrollState::new(ScrollAxis::Horizontal);
        state.set_viewport_size(400.0, 600.0);
        state.set_content_size(800.0, 600.0);
        state.scroll_by(50.0, 0.0);

        let mut commands = velox_scene::CommandList::new();
        let bounds = velox_scene::Rect::new(0.0, 0.0, 400.0, 600.0);
        state.paint_scrollbars(&mut commands, bounds, &ScrollbarColors::default());
        assert!(
            commands.commands().len() >= 2,
            "should emit horizontal track + thumb"
        );
    }
}
