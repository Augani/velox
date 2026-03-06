use velox_scene::{Key, KeyState, Modifiers};
use winit::event::ElementState;
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};

pub fn convert_key(winit_key: &WinitKey) -> Option<Key> {
    match winit_key {
        WinitKey::Character(c) => {
            let ch = c.chars().next()?;
            match ch.to_ascii_lowercase() {
                'a' => Some(Key::A),
                'b' => Some(Key::B),
                'c' => Some(Key::C),
                'd' => Some(Key::D),
                'e' => Some(Key::E),
                'f' => Some(Key::F),
                'g' => Some(Key::G),
                'h' => Some(Key::H),
                'i' => Some(Key::I),
                'j' => Some(Key::J),
                'k' => Some(Key::K),
                'l' => Some(Key::L),
                'm' => Some(Key::M),
                'n' => Some(Key::N),
                'o' => Some(Key::O),
                'p' => Some(Key::P),
                'q' => Some(Key::Q),
                'r' => Some(Key::R),
                's' => Some(Key::S),
                't' => Some(Key::T),
                'u' => Some(Key::U),
                'v' => Some(Key::V),
                'w' => Some(Key::W),
                'x' => Some(Key::X),
                'y' => Some(Key::Y),
                'z' => Some(Key::Z),
                '0' => Some(Key::Num0),
                '1' => Some(Key::Num1),
                '2' => Some(Key::Num2),
                '3' => Some(Key::Num3),
                '4' => Some(Key::Num4),
                '5' => Some(Key::Num5),
                '6' => Some(Key::Num6),
                '7' => Some(Key::Num7),
                '8' => Some(Key::Num8),
                '9' => Some(Key::Num9),
                _ => None,
            }
        }
        WinitKey::Named(named) => match named {
            NamedKey::Enter => Some(Key::Enter),
            NamedKey::Escape => Some(Key::Escape),
            NamedKey::Tab => Some(Key::Tab),
            NamedKey::Space => Some(Key::Space),
            NamedKey::Backspace => Some(Key::Backspace),
            NamedKey::Delete => Some(Key::Delete),
            NamedKey::ArrowUp => Some(Key::ArrowUp),
            NamedKey::ArrowDown => Some(Key::ArrowDown),
            NamedKey::ArrowLeft => Some(Key::ArrowLeft),
            NamedKey::ArrowRight => Some(Key::ArrowRight),
            NamedKey::Home => Some(Key::Home),
            NamedKey::End => Some(Key::End),
            NamedKey::PageUp => Some(Key::PageUp),
            NamedKey::PageDown => Some(Key::PageDown),
            NamedKey::F1 => Some(Key::F1),
            NamedKey::F2 => Some(Key::F2),
            NamedKey::F3 => Some(Key::F3),
            NamedKey::F4 => Some(Key::F4),
            NamedKey::F5 => Some(Key::F5),
            NamedKey::F6 => Some(Key::F6),
            NamedKey::F7 => Some(Key::F7),
            NamedKey::F8 => Some(Key::F8),
            NamedKey::F9 => Some(Key::F9),
            NamedKey::F10 => Some(Key::F10),
            NamedKey::F11 => Some(Key::F11),
            NamedKey::F12 => Some(Key::F12),
            _ => None,
        },
        _ => None,
    }
}

pub fn convert_modifiers(mods: ModifiersState) -> Modifiers {
    let mut result = Modifiers::empty();
    if mods.shift_key() {
        result |= Modifiers::SHIFT;
    }
    if mods.control_key() {
        result |= Modifiers::CTRL;
    }
    if mods.alt_key() {
        result |= Modifiers::ALT;
    }
    if mods.super_key() {
        result |= Modifiers::SUPER;
    }
    result
}

pub fn convert_element_state(state: ElementState) -> KeyState {
    match state {
        ElementState::Pressed => KeyState::Pressed,
        ElementState::Released => KeyState::Released,
    }
}
