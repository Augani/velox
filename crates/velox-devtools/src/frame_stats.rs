use std::collections::VecDeque;
use std::time::{Duration, Instant};

const MAX_SAMPLES: usize = 120;

pub struct FrameStats {
    samples: VecDeque<Duration>,
    last_frame: Option<Instant>,
}

impl FrameStats {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(MAX_SAMPLES),
            last_frame: None,
        }
    }

    pub fn begin_frame(&mut self) {
        let now = Instant::now();
        if let Some(prev) = self.last_frame {
            let dt = now - prev;
            if self.samples.len() >= MAX_SAMPLES {
                self.samples.pop_front();
            }
            self.samples.push_back(dt);
        }
        self.last_frame = Some(now);
    }

    pub fn avg_frame_time(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.samples.iter().copied().sum();
        total / self.samples.len() as u32
    }

    pub fn fps(&self) -> f64 {
        let avg = self.avg_frame_time();
        if avg.is_zero() {
            return 0.0;
        }
        1.0 / avg.as_secs_f64()
    }

    pub fn max_frame_time(&self) -> Duration {
        self.samples.iter().copied().max().unwrap_or(Duration::ZERO)
    }

    pub fn min_frame_time(&self) -> Duration {
        self.samples.iter().copied().min().unwrap_or(Duration::ZERO)
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn samples(&self) -> &VecDeque<Duration> {
        &self.samples
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.last_frame = None;
    }
}

impl Default for FrameStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_zero() {
        let stats = FrameStats::new();
        assert_eq!(stats.fps(), 0.0);
        assert_eq!(stats.avg_frame_time(), Duration::ZERO);
        assert_eq!(stats.sample_count(), 0);
    }

    #[test]
    fn first_begin_frame_records_no_sample() {
        let mut stats = FrameStats::new();
        stats.begin_frame();
        assert_eq!(stats.sample_count(), 0);
    }

    #[test]
    fn subsequent_frames_record_samples() {
        let mut stats = FrameStats::new();
        stats.begin_frame();
        std::thread::sleep(Duration::from_millis(1));
        stats.begin_frame();
        assert_eq!(stats.sample_count(), 1);
        assert!(stats.avg_frame_time() > Duration::ZERO);
        assert!(stats.fps() > 0.0);
    }

    #[test]
    fn max_samples_bounded() {
        let mut stats = FrameStats::new();
        for _ in 0..150 {
            stats.begin_frame();
        }
        assert!(stats.sample_count() <= 120);
    }

    #[test]
    fn reset_clears_state() {
        let mut stats = FrameStats::new();
        stats.begin_frame();
        std::thread::sleep(Duration::from_millis(1));
        stats.begin_frame();
        stats.reset();
        assert_eq!(stats.sample_count(), 0);
        assert_eq!(stats.fps(), 0.0);
    }
}
