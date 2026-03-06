use std::time::Duration;

use crate::easing::Easing;
use crate::interpolate::Interpolatable;

pub struct KeyframeEntry<T> {
    pub time: Duration,
    pub value: T,
    pub easing: Easing,
}

pub struct Keyframes<T: Interpolatable> {
    entries: Vec<KeyframeEntry<T>>,
    elapsed: Duration,
}

impl<T: Interpolatable> Keyframes<T> {
    pub fn new(entries: Vec<KeyframeEntry<T>>) -> Option<Self> {
        if entries.is_empty() {
            return None;
        }
        Some(Self {
            entries,
            elapsed: Duration::ZERO,
        })
    }

    pub fn advance(&mut self, dt: Duration) -> T {
        let total = self.total_duration();
        self.elapsed = (self.elapsed + dt).min(total);
        self.value_at(self.elapsed)
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.total_duration()
    }

    pub fn total_duration(&self) -> Duration {
        self.entries
            .last()
            .map(|e| e.time)
            .unwrap_or(Duration::ZERO)
    }

    fn value_at(&self, time: Duration) -> T {
        if self.entries.len() == 1 || time <= self.entries[0].time {
            return self.entries[0].value.clone();
        }

        for window in self.entries.windows(2) {
            let start = &window[0];
            let end = &window[1];

            if time >= start.time && time <= end.time {
                let segment_duration = end.time.saturating_sub(start.time);
                if segment_duration.is_zero() {
                    return end.value.clone();
                }
                let segment_elapsed = time.saturating_sub(start.time);
                let raw_t = (segment_elapsed.as_secs_f64() / segment_duration.as_secs_f64()) as f32;
                let eased_t = start.easing.apply(raw_t);
                return start.value.lerp(&end.value, eased_t);
            }
        }

        self.entries.last().unwrap().value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_keyframe_interpolation() {
        let entries = vec![
            KeyframeEntry {
                time: Duration::ZERO,
                value: 0.0_f32,
                easing: Easing::Linear,
            },
            KeyframeEntry {
                time: Duration::from_secs(1),
                value: 100.0,
                easing: Easing::Linear,
            },
        ];
        let mut kf = Keyframes::new(entries).unwrap();
        let val = kf.advance(Duration::from_millis(500));
        assert!((val - 50.0).abs() < 1e-3);
    }

    #[test]
    fn finished_detection() {
        let entries = vec![
            KeyframeEntry {
                time: Duration::ZERO,
                value: 0.0_f32,
                easing: Easing::Linear,
            },
            KeyframeEntry {
                time: Duration::from_secs(1),
                value: 100.0,
                easing: Easing::Linear,
            },
        ];
        let mut kf = Keyframes::new(entries).unwrap();
        assert!(!kf.is_finished());
        kf.advance(Duration::from_secs(2));
        assert!(kf.is_finished());
    }

    #[test]
    fn total_duration_correct() {
        let entries = vec![
            KeyframeEntry {
                time: Duration::ZERO,
                value: 0.0_f32,
                easing: Easing::Linear,
            },
            KeyframeEntry {
                time: Duration::from_millis(500),
                value: 50.0,
                easing: Easing::Linear,
            },
            KeyframeEntry {
                time: Duration::from_secs(2),
                value: 100.0,
                easing: Easing::Linear,
            },
        ];
        let kf = Keyframes::new(entries).unwrap();
        assert_eq!(kf.total_duration(), Duration::from_secs(2));
    }
}
