use std::time::Duration;
use velox_runtime::FrameClock;

#[test]
fn frame_clock_starts_at_zero() {
    let clock = FrameClock::new();
    assert_eq!(clock.frame_count(), 0);
}

#[test]
fn frame_clock_increments_on_tick() {
    let mut clock = FrameClock::new();
    clock.tick();
    assert_eq!(clock.frame_count(), 1);
    clock.tick();
    assert_eq!(clock.frame_count(), 2);
}

#[test]
fn frame_clock_tracks_delta() {
    let mut clock = FrameClock::new();
    std::thread::sleep(Duration::from_millis(16));
    clock.tick();
    let delta = clock.delta_time();
    assert!(delta >= Duration::from_millis(10));
    assert!(delta < Duration::from_millis(100));
}
