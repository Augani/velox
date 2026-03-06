use std::path::PathBuf;

use crate::{
    app::PlatformApp,
    clipboard::PlatformClipboard,
    file_dialog::PlatformFileDialog,
    menu::{MenuItem, PlatformMenu},
    notification::PlatformNotification,
    power::{BatteryState, PlatformPower, PowerSource},
    tray::PlatformTray,
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

impl PlatformFileDialog for StubPlatform {
    fn open_file(&self, _title: &str, _filters: &[(&str, &[&str])]) -> Option<PathBuf> {
        None
    }

    fn save_file(
        &self,
        _title: &str,
        _default_name: &str,
        _filters: &[(&str, &[&str])],
    ) -> Option<PathBuf> {
        None
    }

    fn open_directory(&self, _title: &str) -> Option<PathBuf> {
        None
    }
}

impl PlatformNotification for StubPlatform {
    fn show(&self, _title: &str, _body: &str) -> Result<(), String> {
        Ok(())
    }
}

impl PlatformTray for StubPlatform {
    fn set_icon(&mut self, _icon_data: &[u8], _width: u32, _height: u32) -> Result<(), String> {
        Ok(())
    }

    fn set_tooltip(&mut self, _tooltip: &str) -> Result<(), String> {
        Ok(())
    }

    fn set_visible(&mut self, _visible: bool) -> Result<(), String> {
        Ok(())
    }
}

impl PlatformMenu for StubPlatform {
    fn set_items(&mut self, _items: Vec<MenuItem>) -> Result<(), String> {
        Ok(())
    }

    fn set_item_enabled(&mut self, _id: &str, _enabled: bool) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_file_dialog_returns_none() {
        let stub = StubPlatform;
        assert!(stub.open_file("Open", &[]).is_none());
        assert!(stub.save_file("Save", "f.txt", &[]).is_none());
        assert!(stub.open_directory("Dir").is_none());
    }

    #[test]
    fn stub_notification_returns_ok() {
        let stub = StubPlatform;
        assert!(PlatformNotification::show(&stub, "Title", "Body").is_ok());
    }

    #[test]
    fn stub_tray_returns_ok() {
        let mut stub = StubPlatform;
        assert!(stub.set_icon(&[0], 1, 1).is_ok());
        assert!(stub.set_tooltip("tip").is_ok());
        assert!(stub.set_visible(true).is_ok());
    }

    #[test]
    fn stub_menu_returns_ok() {
        let mut stub = StubPlatform;
        assert!(stub.set_items(vec![]).is_ok());
        assert!(stub.set_item_enabled("x", true).is_ok());
    }
}
