use std::sync::Arc;
use velox_scene::{KeyEvent, MouseEvent, Point, ScrollEvent};

pub type ClickHandler = Arc<dyn Fn(&ClickEvent) + 'static>;
pub type MouseHandler = Arc<dyn Fn(&MouseEvent) + 'static>;
pub type ScrollHandler = Arc<dyn Fn(&ScrollEvent) + 'static>;
pub type KeyHandler = Arc<dyn Fn(&KeyEvent) + 'static>;
pub type HoverHandler = Arc<dyn Fn(bool) + 'static>;
pub type FocusHandler = Arc<dyn Fn(bool) + 'static>;

#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub position: Point,
    pub button: velox_scene::MouseButton,
    pub click_count: u32,
}

#[derive(Default)]
pub struct EventHandlers {
    pub on_click: Option<ClickHandler>,
    pub on_mouse_down: Option<MouseHandler>,
    pub on_mouse_up: Option<MouseHandler>,
    pub on_hover: Option<HoverHandler>,
    pub on_scroll: Option<ScrollHandler>,
    pub on_key_down: Option<KeyHandler>,
    pub on_focus: Option<FocusHandler>,
    pub focusable: bool,
}

pub trait InteractiveElement: Sized {
    fn handlers_mut(&mut self) -> &mut EventHandlers;

    fn on_click(mut self, handler: impl Fn(&ClickEvent) + 'static) -> Self {
        self.handlers_mut().on_click = Some(Arc::new(handler));
        self
    }

    fn on_mouse_down(mut self, handler: impl Fn(&MouseEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_down = Some(Arc::new(handler));
        self
    }

    fn on_mouse_up(mut self, handler: impl Fn(&MouseEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_up = Some(Arc::new(handler));
        self
    }

    fn on_hover(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.handlers_mut().on_hover = Some(Arc::new(handler));
        self
    }

    fn on_scroll(mut self, handler: impl Fn(&ScrollEvent) + 'static) -> Self {
        self.handlers_mut().on_scroll = Some(Arc::new(handler));
        self
    }

    fn on_key_down(mut self, handler: impl Fn(&KeyEvent) + 'static) -> Self {
        self.handlers_mut().on_key_down = Some(Arc::new(handler));
        self
    }

    fn on_focus(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.handlers_mut().on_focus = Some(Arc::new(handler));
        self
    }

    fn focusable(mut self) -> Self {
        self.handlers_mut().focusable = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestInteractive {
        handlers: EventHandlers,
    }
    impl TestInteractive {
        fn new() -> Self {
            Self {
                handlers: EventHandlers::default(),
            }
        }
    }
    impl InteractiveElement for TestInteractive {
        fn handlers_mut(&mut self) -> &mut EventHandlers {
            &mut self.handlers
        }
    }

    #[test]
    fn on_click_registers_handler() {
        let el = TestInteractive::new().on_click(|_| {});
        assert!(el.handlers.on_click.is_some());
    }

    #[test]
    fn focusable_sets_flag() {
        let el = TestInteractive::new().focusable();
        assert!(el.handlers.focusable);
    }

    #[test]
    fn default_handlers_all_none() {
        let h = EventHandlers::default();
        assert!(h.on_click.is_none());
        assert!(h.on_mouse_down.is_none());
        assert!(!h.focusable);
    }
}
