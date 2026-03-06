use std::time::{Duration, Instant};

pub struct FrameClock {
    frame_count: u64,
    last_tick: Instant,
    delta: Duration,
    start_time: Instant,
}

impl FrameClock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_count: 0,
            last_tick: now,
            delta: Duration::ZERO,
            start_time: now,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.delta = now - self.last_tick;
        self.last_tick = now;
        self.frame_count += 1;
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn delta_time(&self) -> Duration {
        self.delta
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}
