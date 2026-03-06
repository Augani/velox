use velox_reactive::{Signal, Subscription};

use crate::Theme;

#[derive(Clone)]
pub struct ThemeManager {
    active: Signal<Theme>,
}

impl ThemeManager {
    pub fn new(theme: Theme) -> Self {
        Self {
            active: Signal::new(theme),
        }
    }

    pub fn current(&self) -> Theme {
        self.active.get()
    }

    pub fn set_theme(&self, theme: Theme) {
        if self.active.get() == theme {
            return;
        }
        self.active.set(theme);
    }

    pub fn update(&self, f: impl FnOnce(&mut Theme)) {
        self.active.update(f);
    }

    pub fn subscribe(&self, callback: impl Fn(&Theme) + 'static) -> Subscription {
        self.active.subscribe(callback)
    }

    pub fn version(&self) -> u64 {
        self.active.version()
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new(Theme::light())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn default_uses_light_theme() {
        let manager = ThemeManager::default();
        assert_eq!(manager.current().name, "light");
    }

    #[test]
    fn set_theme_notifies_subscribers() {
        let manager = ThemeManager::default();
        let notifications = Rc::new(Cell::new(0u32));
        let n = notifications.clone();
        let _sub = manager.subscribe(move |_| {
            n.set(n.get() + 1);
        });

        manager.set_theme(Theme::dark());
        assert_eq!(notifications.get(), 1);
    }

    #[test]
    fn setting_same_theme_is_noop() {
        let manager = ThemeManager::default();
        let notifications = Rc::new(Cell::new(0u32));
        let n = notifications.clone();
        let _sub = manager.subscribe(move |_| {
            n.set(n.get() + 1);
        });

        manager.set_theme(Theme::light());
        assert_eq!(notifications.get(), 0);
    }

    #[test]
    fn version_increments_on_change() {
        let manager = ThemeManager::default();
        let initial = manager.version();
        manager.set_theme(Theme::dark());
        assert!(manager.version() > initial);
    }
}
