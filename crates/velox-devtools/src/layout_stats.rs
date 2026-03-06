use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct LayoutStats {
    durations: VecDeque<Duration>,
    max_samples: usize,
}

impl LayoutStats {
    pub fn new(max_samples: usize) -> Self {
        let clamped = max_samples.max(1);
        Self {
            durations: VecDeque::with_capacity(clamped),
            max_samples: clamped,
        }
    }

    pub fn begin_layout() -> Instant {
        Instant::now()
    }

    pub fn end_layout(&mut self, start: Instant) {
        let elapsed = start.elapsed();
        if self.durations.len() >= self.max_samples {
            self.durations.pop_front();
        }
        self.durations.push_back(elapsed);
    }

    pub fn average_duration(&self) -> Duration {
        if self.durations.is_empty() {
            return Duration::ZERO;
        }
        let total: Duration = self.durations.iter().copied().sum();
        total / self.durations.len() as u32
    }

    pub fn max_duration(&self) -> Duration {
        self.durations
            .iter()
            .copied()
            .max()
            .unwrap_or(Duration::ZERO)
    }

    pub fn last_duration(&self) -> Option<Duration> {
        self.durations.back().copied()
    }

    pub fn sample_count(&self) -> usize {
        self.durations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_empty() {
        let stats = LayoutStats::new(10);
        assert_eq!(stats.sample_count(), 0);
        assert_eq!(stats.average_duration(), Duration::ZERO);
        assert_eq!(stats.max_duration(), Duration::ZERO);
        assert!(stats.last_duration().is_none());
    }

    #[test]
    fn records_and_computes_average() {
        let mut stats = LayoutStats::new(10);
        let start = LayoutStats::begin_layout();
        std::thread::sleep(Duration::from_millis(1));
        stats.end_layout(start);
        assert_eq!(stats.sample_count(), 1);
        assert!(stats.average_duration() > Duration::ZERO);
        assert!(stats.last_duration().is_some());
    }

    #[test]
    fn max_samples_bounded() {
        let mut stats = LayoutStats::new(3);
        for _ in 0..5 {
            let start = Instant::now();
            stats.end_layout(start);
        }
        assert_eq!(stats.sample_count(), 3);
    }

    #[test]
    fn max_duration_correct() {
        let mut stats = LayoutStats::new(10);

        let start = Instant::now();
        stats.end_layout(start);

        let start2 = Instant::now();
        std::thread::sleep(Duration::from_millis(2));
        stats.end_layout(start2);

        assert!(stats.max_duration() >= stats.average_duration());
    }
}
