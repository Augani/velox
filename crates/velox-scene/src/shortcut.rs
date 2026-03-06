use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0001;
        const CTRL  = 0b0010;
        const ALT   = 0b0100;
        const SUPER = 0b1000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyCombo {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShortcutId(u64);

type ShortcutEntry = (KeyCombo, Box<dyn FnMut()>);

pub struct ShortcutRegistry {
    shortcuts: HashMap<ShortcutId, ShortcutEntry>,
    next_id: u64,
}

impl ShortcutRegistry {
    pub fn new() -> Self {
        Self {
            shortcuts: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn register(&mut self, combo: KeyCombo, callback: impl FnMut() + 'static) -> ShortcutId {
        let id = ShortcutId(self.next_id);
        self.next_id += 1;
        self.shortcuts.insert(id, (combo, Box::new(callback)));
        id
    }

    pub fn unregister(&mut self, id: ShortcutId) {
        self.shortcuts.remove(&id);
    }

    pub fn handle_key_event(&mut self, key: Key, modifiers: Modifiers) -> bool {
        let combo = KeyCombo { key, modifiers };
        for (registered, callback) in self.shortcuts.values_mut() {
            if *registered == combo {
                callback();
                return true;
            }
        }
        false
    }
}

impl Default for ShortcutRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn register_and_fire_shortcut() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(KeyCombo::new(Key::S, Modifiers::SUPER), move || f.set(true));
        let handled = registry.handle_key_event(Key::S, Modifiers::SUPER);
        assert!(handled);
        assert!(fired.get());
    }

    #[test]
    fn unmatched_key_returns_false() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(KeyCombo::new(Key::S, Modifiers::SUPER), move || f.set(true));
        let handled = registry.handle_key_event(Key::Q, Modifiers::SUPER);
        assert!(!handled);
        assert!(!fired.get());
    }

    #[test]
    fn unregister_shortcut() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        let id = registry.register(KeyCombo::new(Key::W, Modifiers::SUPER), move || f.set(true));
        registry.unregister(id);
        let handled = registry.handle_key_event(Key::W, Modifiers::SUPER);
        assert!(!handled);
        assert!(!fired.get());
    }

    #[test]
    fn modifier_must_match() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(KeyCombo::new(Key::S, Modifiers::CTRL), move || f.set(true));
        let handled = registry.handle_key_event(Key::S, Modifiers::SUPER);
        assert!(!handled);
        assert!(!fired.get());
    }

    #[test]
    fn multiple_modifiers() {
        let mut registry = ShortcutRegistry::new();
        let fired = Rc::new(Cell::new(false));
        let f = fired.clone();
        registry.register(
            KeyCombo::new(Key::S, Modifiers::CTRL | Modifiers::SHIFT),
            move || f.set(true),
        );
        let handled = registry.handle_key_event(Key::S, Modifiers::CTRL | Modifiers::SHIFT);
        assert!(handled);
        assert!(fired.get());
    }
}
