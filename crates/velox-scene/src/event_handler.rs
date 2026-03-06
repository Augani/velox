use crate::event::{KeyEvent, MouseEvent, ScrollEvent};
use crate::geometry::Rect;

pub struct EventContext {
    rect: Rect,
    redraw: bool,
    clipboard_read: Option<String>,
    clipboard_write: Option<String>,
}

impl EventContext {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            redraw: false,
            clipboard_read: None,
            clipboard_write: None,
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn request_redraw(&mut self) {
        self.redraw = true;
    }

    pub fn redraw_requested(&self) -> bool {
        self.redraw
    }

    pub fn set_clipboard_content(&mut self, content: Option<String>) {
        self.clipboard_read = content;
    }

    pub fn clipboard_get(&self) -> Option<&str> {
        self.clipboard_read.as_deref()
    }

    pub fn clipboard_set(&mut self, text: &str) {
        self.clipboard_write = Some(text.to_owned());
    }

    pub fn take_clipboard_write(&mut self) -> Option<String> {
        self.clipboard_write.take()
    }
}

pub trait EventHandler: 'static {
    fn handle_key(&mut self, event: &KeyEvent, ctx: &mut EventContext) -> bool;

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &mut EventContext) -> bool {
        let _ = (event, ctx);
        false
    }

    fn handle_scroll(&mut self, event: &ScrollEvent, ctx: &mut EventContext) -> bool {
        let _ = (event, ctx);
        false
    }

    fn handle_ime(&mut self, event: &crate::ime::ImeEvent, ctx: &mut EventContext) -> bool {
        let _ = (event, ctx);
        false
    }

    fn handle_focus(&mut self, gained: bool) {
        let _ = gained;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ButtonState, KeyEvent, KeyState, MouseButton, MouseEvent};
    use crate::geometry::{Point, Rect};
    use crate::shortcut::{Key, Modifiers};
    use std::cell::Cell;
    use std::rc::Rc;

    struct TestKeyHandler {
        handled: Rc<Cell<bool>>,
    }

    impl EventHandler for TestKeyHandler {
        fn handle_key(&mut self, _event: &KeyEvent, _ctx: &mut EventContext) -> bool {
            self.handled.set(true);
            true
        }
    }

    #[test]
    fn event_handler_receives_key() {
        let handled = Rc::new(Cell::new(false));
        let mut handler = TestKeyHandler {
            handled: handled.clone(),
        };
        let mut ctx = EventContext::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        let event = KeyEvent {
            key: Key::A,
            modifiers: Modifiers::empty(),
            state: KeyState::Pressed,
            text: Some("a".into()),
        };
        let consumed = handler.handle_key(&event, &mut ctx);
        assert!(consumed);
        assert!(handled.get());
    }

    #[test]
    fn event_context_request_redraw() {
        let mut ctx = EventContext::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert!(!ctx.redraw_requested());
        ctx.request_redraw();
        assert!(ctx.redraw_requested());
    }

    #[test]
    fn default_mouse_handler_returns_false() {
        struct EmptyHandler;
        impl EventHandler for EmptyHandler {
            fn handle_key(&mut self, _: &KeyEvent, _: &mut EventContext) -> bool {
                false
            }
        }
        let mut handler = EmptyHandler;
        let mut ctx = EventContext::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        let event = MouseEvent {
            position: Point::new(10.0, 10.0),
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            click_count: 1,
            modifiers: Modifiers::empty(),
        };
        assert!(!handler.handle_mouse(&event, &mut ctx));
    }
}
