use std::time::Duration;

use crate::easing::Easing;
use crate::tween::Tween;

pub struct Crossfade {
    fade_out: Tween<f32>,
    fade_in: Tween<f32>,
}

impl Crossfade {
    pub fn new(duration: Duration, easing: Easing) -> Self {
        Self {
            fade_out: Tween::new(1.0, 0.0, duration, easing.clone()),
            fade_in: Tween::new(0.0, 1.0, duration, easing),
        }
    }

    pub fn advance(&mut self, dt: Duration) -> (f32, f32) {
        let out = self.fade_out.advance(dt);
        let incoming = self.fade_in.advance(dt);
        (out, incoming)
    }

    pub fn is_finished(&self) -> bool {
        self.fade_out.is_finished() && self.fade_in.is_finished()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_at_start() {
        let mut cf = Crossfade::new(Duration::from_secs(1), Easing::Linear);
        let (out, incoming) = cf.advance(Duration::ZERO);
        assert!((out - 1.0).abs() < 1e-6);
        assert!((incoming - 0.0).abs() < 1e-6);
    }

    #[test]
    fn opacity_at_end() {
        let mut cf = Crossfade::new(Duration::from_secs(1), Easing::Linear);
        let (out, incoming) = cf.advance(Duration::from_secs(1));
        assert!((out - 0.0).abs() < 1e-6);
        assert!((incoming - 1.0).abs() < 1e-6);
    }

    #[test]
    fn advance_to_completion() {
        let mut cf = Crossfade::new(Duration::from_secs(1), Easing::Linear);
        cf.advance(Duration::from_secs(2));
        assert!(cf.is_finished());
    }
}
