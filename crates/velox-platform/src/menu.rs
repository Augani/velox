#[derive(Debug, Clone)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

impl MenuItem {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

pub trait PlatformMenu {
    fn set_items(&mut self, items: Vec<MenuItem>) -> Result<(), String>;
    fn set_item_enabled(&mut self, id: &str, enabled: bool) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubMenu;
    impl PlatformMenu for StubMenu {
        fn set_items(&mut self, _items: Vec<MenuItem>) -> Result<(), String> {
            Ok(())
        }
        fn set_item_enabled(&mut self, _id: &str, _enabled: bool) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn menu_item_creation() {
        let item = MenuItem::new("file_open", "Open File");
        assert_eq!(item.id, "file_open");
        assert_eq!(item.label, "Open File");
        assert!(item.enabled);
    }

    #[test]
    fn menu_item_disabled() {
        let item = MenuItem::new("paste", "Paste").disabled();
        assert!(!item.enabled);
    }

    #[test]
    fn stub_menu_returns_ok() {
        let mut menu = StubMenu;
        assert!(menu.set_items(vec![MenuItem::new("a", "A")]).is_ok());
        assert!(menu.set_item_enabled("a", false).is_ok());
    }
}
