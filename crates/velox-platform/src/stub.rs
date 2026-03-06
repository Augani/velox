use crate::{
    app::PlatformApp,
    clipboard::PlatformClipboard,
    power::{BatteryState, PlatformPower, PowerSource},
};

pub struct StubPlatform;

impl StubPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformPower for StubPlatform {
    fn battery_state(&self) -> BatteryState {
        BatteryState::Unknown
    }

    fn power_source(&self) -> PowerSource {
        PowerSource::Unknown
    }

    fn is_low_power_mode(&self) -> bool {
        false
    }
}

impl PlatformApp for StubPlatform {
    fn hide(&self) {}

    fn show(&self) {}

    fn set_badge(&self, _text: Option<&str>) {}
}

impl PlatformClipboard for StubPlatform {
    fn read_text(&self) -> Option<String> {
        None
    }

    fn write_text(&self, _text: &str) {}
}
