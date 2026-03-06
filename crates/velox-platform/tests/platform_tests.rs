use velox_platform::{BatteryState, PlatformPower, PowerSource, StubPlatform};

#[test]
fn stub_power_returns_defaults() {
    let platform = StubPlatform::new();
    assert!(matches!(platform.battery_state(), BatteryState::Unknown));
    assert!(matches!(platform.power_source(), PowerSource::Unknown));
    assert!(!platform.is_low_power_mode());
}
