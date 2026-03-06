use crate::geometry::Point;
use crate::shortcut::{Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

impl KeyState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}

#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub state: KeyState,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}

#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub position: Point,
    pub button: MouseButton,
    pub state: ButtonState,
    pub click_count: u32,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollEvent {
    pub delta_x: f32,
    pub delta_y: f32,
    pub modifiers: Modifiers,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcut::{Key, Modifiers};

    #[test]
    fn key_event_with_text() {
        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: KeyState::Pressed,
            text: Some("a".into()),
        };
        assert_eq!(event.text.as_deref(), Some("a"));
        assert!(event.state.is_pressed());
    }

    #[test]
    fn key_event_modifier_only() {
        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::CTRL,
            state: KeyState::Pressed,
            text: None,
        };
        assert!(event.modifiers.contains(Modifiers::CTRL));
        assert!(event.text.is_none());
    }

    #[test]
    fn mouse_event_click() {
        let event = MouseEvent {
            position: crate::Point::new(50.0, 30.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 1,
            modifiers: Modifiers::empty(),
        };
        assert_eq!(event.click_count, 1);
        assert!(event.state.is_pressed());
    }

    #[test]
    fn mouse_event_double_click() {
        let event = MouseEvent {
            position: crate::Point::new(50.0, 30.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 2,
            modifiers: Modifiers::empty(),
        };
        assert_eq!(event.click_count, 2);
    }
}
