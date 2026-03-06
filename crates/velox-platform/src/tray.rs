pub trait PlatformTray {
    fn set_icon(&mut self, icon_data: &[u8], width: u32, height: u32) -> Result<(), String>;
    fn set_tooltip(&mut self, tooltip: &str) -> Result<(), String>;
    fn set_visible(&mut self, visible: bool) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubTray;
    impl PlatformTray for StubTray {
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

    #[test]
    fn stub_tray_returns_ok() {
        let mut tray = StubTray;
        assert!(tray.set_icon(&[0, 0, 0, 255], 1, 1).is_ok());
        assert!(tray.set_tooltip("Velox").is_ok());
        assert!(tray.set_visible(true).is_ok());
    }
}
