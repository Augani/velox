use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;
use velox_scene::{KeyEvent, Modifiers, MouseEvent, Point, ScrollEvent};

use crate::style::Style;

pub type ClickHandler = Rc<dyn Fn(&ClickEvent) + 'static>;
pub type MouseHandler = Rc<dyn Fn(&MouseEvent) + 'static>;
pub type MouseMoveHandler = Rc<dyn Fn(&MouseMoveEvent) + 'static>;
pub type ScrollHandler = Rc<dyn Fn(&ScrollEvent) + 'static>;
pub type KeyHandler = Rc<dyn Fn(&KeyEvent) + 'static>;
pub type HoverHandler = Rc<dyn Fn(bool) + 'static>;
pub type FocusHandler = Rc<dyn Fn(bool) + 'static>;
pub type DragHandler = Rc<dyn Fn() -> Arc<dyn Any + Send + Sync> + 'static>;
pub type DropHandler = Rc<dyn Fn(&dyn Any) + 'static>;
pub type ActionHandlerFn = Rc<dyn Fn(&dyn Any) + 'static>;

#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub position: Point,
    pub button: velox_scene::MouseButton,
    pub click_count: u32,
}

#[derive(Debug, Clone)]
pub struct MouseMoveEvent {
    pub position: Point,
    pub modifiers: Modifiers,
}

#[derive(Default)]
pub struct EventHandlers {
    pub on_click: Option<ClickHandler>,
    pub on_mouse_down: Option<MouseHandler>,
    pub on_mouse_up: Option<MouseHandler>,
    pub on_mouse_move: Option<MouseMoveHandler>,
    pub on_hover: Option<HoverHandler>,
    pub on_scroll: Option<ScrollHandler>,
    pub on_key_down: Option<KeyHandler>,
    pub capture_on_key_down: Option<KeyHandler>,
    pub on_focus: Option<FocusHandler>,
    pub on_drag: Option<DragHandler>,
    pub on_drop: Vec<(TypeId, DropHandler)>,
    pub on_action: Vec<(TypeId, ActionHandlerFn)>,
    #[allow(clippy::type_complexity)]
    pub can_drop: Vec<(TypeId, Rc<dyn Fn(&dyn Any) -> bool + 'static>)>,
    pub drag_over_styles: Vec<(TypeId, Rc<dyn Fn() -> Style + 'static>)>,
    pub key_context: Option<String>,
    pub tab_index: Option<i32>,
    pub focusable: bool,
}

impl Clone for EventHandlers {
    fn clone(&self) -> Self {
        Self {
            on_click: self.on_click.clone(),
            on_mouse_down: self.on_mouse_down.clone(),
            on_mouse_up: self.on_mouse_up.clone(),
            on_mouse_move: self.on_mouse_move.clone(),
            on_hover: self.on_hover.clone(),
            on_scroll: self.on_scroll.clone(),
            on_key_down: self.on_key_down.clone(),
            capture_on_key_down: self.capture_on_key_down.clone(),
            on_focus: self.on_focus.clone(),
            on_drag: self.on_drag.clone(),
            on_drop: self.on_drop.clone(),
            on_action: self.on_action.clone(),
            can_drop: self.can_drop.clone(),
            drag_over_styles: self.drag_over_styles.clone(),
            key_context: self.key_context.clone(),
            tab_index: self.tab_index,
            focusable: self.focusable,
        }
    }
}

pub trait InteractiveElement: Sized {
    fn handlers_mut(&mut self) -> &mut EventHandlers;

    fn on_click(mut self, handler: impl Fn(&ClickEvent) + 'static) -> Self {
        self.handlers_mut().on_click = Some(Rc::new(handler));
        self
    }

    fn on_mouse_down(mut self, handler: impl Fn(&MouseEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_down = Some(Rc::new(handler));
        self
    }

    fn on_mouse_up(mut self, handler: impl Fn(&MouseEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_up = Some(Rc::new(handler));
        self
    }

    fn on_hover(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.handlers_mut().on_hover = Some(Rc::new(handler));
        self
    }

    fn on_scroll(mut self, handler: impl Fn(&ScrollEvent) + 'static) -> Self {
        self.handlers_mut().on_scroll = Some(Rc::new(handler));
        self
    }

    fn on_key_down(mut self, handler: impl Fn(&KeyEvent) + 'static) -> Self {
        self.handlers_mut().on_key_down = Some(Rc::new(handler));
        self
    }

    fn on_focus(mut self, handler: impl Fn(bool) + 'static) -> Self {
        self.handlers_mut().on_focus = Some(Rc::new(handler));
        self
    }

    fn on_mouse_move(mut self, handler: impl Fn(&MouseMoveEvent) + 'static) -> Self {
        self.handlers_mut().on_mouse_move = Some(Rc::new(handler));
        self
    }

    fn capture_key_down(mut self, handler: impl Fn(&KeyEvent) + 'static) -> Self {
        self.handlers_mut().capture_on_key_down = Some(Rc::new(handler));
        self
    }

    fn on_drag(mut self, handler: impl Fn() -> Arc<dyn Any + Send + Sync> + 'static) -> Self {
        self.handlers_mut().on_drag = Some(Rc::new(handler));
        self
    }

    fn on_drop_typed<T: Any + 'static>(mut self, handler: impl Fn(&T) + 'static) -> Self {
        let type_id = TypeId::of::<T>();
        let wrapped: DropHandler = Rc::new(move |any: &dyn Any| {
            if let Some(val) = any.downcast_ref::<T>() {
                handler(val);
            }
        });
        self.handlers_mut().on_drop.push((type_id, wrapped));
        self
    }

    fn on_action_typed<A: Any + 'static>(mut self, handler: impl Fn(&A) + 'static) -> Self {
        let type_id = TypeId::of::<A>();
        let wrapped: ActionHandlerFn = Rc::new(move |any: &dyn Any| {
            if let Some(val) = any.downcast_ref::<A>() {
                handler(val);
            }
        });
        self.handlers_mut().on_action.push((type_id, wrapped));
        self
    }

    #[allow(clippy::type_complexity)]
    fn can_drop_typed<T: Any + 'static>(
        mut self,
        predicate: impl Fn(&T) -> bool + 'static,
    ) -> Self {
        let type_id = TypeId::of::<T>();
        let wrapped: Rc<dyn Fn(&dyn Any) -> bool + 'static> =
            Rc::new(move |any: &dyn Any| any.downcast_ref::<T>().is_some_and(&predicate));
        self.handlers_mut().can_drop.push((type_id, wrapped));
        self
    }

    fn drag_over_typed<T: Any + 'static>(mut self, style_fn: impl Fn() -> Style + 'static) -> Self {
        let type_id = TypeId::of::<T>();
        self.handlers_mut()
            .drag_over_styles
            .push((type_id, Rc::new(style_fn)));
        self
    }

    fn key_context(mut self, context: impl Into<String>) -> Self {
        self.handlers_mut().key_context = Some(context.into());
        self
    }

    fn tab_index(mut self, index: i32) -> Self {
        self.handlers_mut().tab_index = Some(index);
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
        assert!(h.on_mouse_move.is_none());
        assert!(h.on_drag.is_none());
        assert!(h.on_drop.is_empty());
        assert!(h.on_action.is_empty());
        assert!(h.capture_on_key_down.is_none());
        assert!(h.key_context.is_none());
        assert!(h.tab_index.is_none());
        assert!(h.can_drop.is_empty());
        assert!(h.drag_over_styles.is_empty());
        assert!(!h.focusable);
    }

    #[test]
    fn on_mouse_move_registers() {
        let el = TestInteractive::new().on_mouse_move(|_| {});
        assert!(el.handlers.on_mouse_move.is_some());
    }

    #[test]
    fn capture_key_down_registers() {
        let el = TestInteractive::new().capture_key_down(|_| {});
        assert!(el.handlers.capture_on_key_down.is_some());
    }

    #[test]
    fn on_drag_registers() {
        let el = TestInteractive::new().on_drag(|| Arc::new(42u32));
        assert!(el.handlers.on_drag.is_some());
    }

    #[test]
    fn on_drop_typed_registers() {
        let el = TestInteractive::new().on_drop_typed::<String>(|_val| {});
        assert_eq!(el.handlers.on_drop.len(), 1);
        assert_eq!(el.handlers.on_drop[0].0, TypeId::of::<String>());
    }

    #[test]
    fn on_action_typed_registers() {
        let el = TestInteractive::new().on_action_typed::<u32>(|_val| {});
        assert_eq!(el.handlers.on_action.len(), 1);
        assert_eq!(el.handlers.on_action[0].0, TypeId::of::<u32>());
    }

    #[test]
    fn can_drop_typed_registers() {
        let el = TestInteractive::new().can_drop_typed::<String>(|_val| true);
        assert_eq!(el.handlers.can_drop.len(), 1);
        assert_eq!(el.handlers.can_drop[0].0, TypeId::of::<String>());
    }

    #[test]
    fn drag_over_typed_registers() {
        let el = TestInteractive::new().drag_over_typed::<String>(|| Style::default());
        assert_eq!(el.handlers.drag_over_styles.len(), 1);
        assert_eq!(el.handlers.drag_over_styles[0].0, TypeId::of::<String>());
    }

    #[test]
    fn key_context_sets_value() {
        let el = TestInteractive::new().key_context("Editor");
        assert_eq!(el.handlers.key_context.as_deref(), Some("Editor"));
    }

    #[test]
    fn tab_index_sets_value() {
        let el = TestInteractive::new().tab_index(5);
        assert_eq!(el.handlers.tab_index, Some(5));
    }

    #[test]
    fn chaining_multiple_handlers() {
        let el = TestInteractive::new()
            .on_click(|_| {})
            .on_mouse_move(|_| {})
            .on_drag(|| Arc::new("item"))
            .on_drop_typed::<String>(|_| {})
            .key_context("Panel")
            .tab_index(1)
            .focusable();
        assert!(el.handlers.on_click.is_some());
        assert!(el.handlers.on_mouse_move.is_some());
        assert!(el.handlers.on_drag.is_some());
        assert_eq!(el.handlers.on_drop.len(), 1);
        assert_eq!(el.handlers.key_context.as_deref(), Some("Panel"));
        assert_eq!(el.handlers.tab_index, Some(1));
        assert!(el.handlers.focusable);
    }
}
