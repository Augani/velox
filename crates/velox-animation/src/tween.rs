use std::time::Duration;

use crate::easing::Easing;
use crate::interpolate::Interpolatable;

pub struct Tween<T: Interpolatable> {
    from: T,
    to: T,
    duration: Duration,
    elapsed: Duration,
    easing: Easing,
}

impl<T: Interpolatable> Tween<T> {
    pub fn new(from: T, to: T, duration: Duration, easing: Easing) -> Self {
        Self {
            from,
            to,
            duration: duration.max(Duration::from_nanos(1)),
            elapsed: Duration::ZERO,
            easing,
        }
    }

    pub fn advance(&mut self, dt: Duration) -> T {
        self.elapsed = (self.elapsed + dt).min(self.duration);
        self.value()
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    pub fn value(&self) -> T {
        let raw = self.progress();
        let eased = self.easing.apply(raw);
        self.from.lerp(&self.to, eased)
    }

    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        (self.elapsed.as_secs_f64() / self.duration.as_secs_f64()) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_to_completion() {
        let mut tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(1), Easing::Linear);
        tween.advance(Duration::from_secs(1));
        assert!(tween.is_finished());
        assert!((tween.value() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn is_finished_before_complete() {
        let tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(1), Easing::Linear);
        assert!(!tween.is_finished());
    }

    #[test]
    fn progress_tracking() {
        let mut tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(2), Easing::Linear);
        tween.advance(Duration::from_secs(1));
        assert!((tween.progress() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn value_at_midpoint() {
        let mut tween = Tween::new(0.0_f32, 100.0, Duration::from_secs(2), Easing::Linear);
        let val = tween.advance(Duration::from_secs(1));
        assert!((val - 50.0).abs() < 1e-6);
    }
}
