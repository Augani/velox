pub trait PlatformNotification {
    fn show(&self, title: &str, body: &str) -> Result<(), String>;
}

pub struct NativeNotification;

impl NativeNotification {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NativeNotification {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformNotification for NativeNotification {
    fn show(&self, title: &str, body: &str) -> Result<(), String> {
        notify_rust::Notification::new()
            .summary(title)
            .body(body)
            .show()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubNotif;
    impl PlatformNotification for StubNotif {
        fn show(&self, _title: &str, _body: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn stub_returns_ok() {
        let notif = StubNotif;
        assert!(notif.show("Test", "Hello").is_ok());
    }
}
