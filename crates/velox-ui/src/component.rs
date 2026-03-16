use std::cell::Cell;
use std::rc::Rc;

use crate::element::AnyElement;
use velox_reactive::Subscription;
use velox_style::Theme;

pub struct ViewContext<'a> {
    theme: &'a Theme,
}

impl<'a> ViewContext<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    pub fn theme(&self) -> &Theme {
        self.theme
    }
}

pub trait Component: 'static + Sized {
    fn render(&self, cx: &ViewContext) -> AnyElement;
}

pub trait Render: Sized {
    fn render(self) -> AnyElement;
}

#[macro_export]
macro_rules! impl_component {
    ($($name:ty),+ $(,)?) => {
        $(
            impl $crate::parent::IntoAnyElement for $name {
                fn into_any_element(self) -> $crate::element::AnyElement {
                    $crate::component::Render::render(self)
                }
            }
        )+
    };
}

pub struct ComponentHost<C: Component> {
    component: C,
    subscriptions: Vec<Subscription>,
    dirty: Rc<Cell<bool>>,
}

impl<C: Component> ComponentHost<C> {
    pub fn new(component: C) -> Self {
        Self {
            component,
            subscriptions: Vec::new(),
            dirty: Rc::new(Cell::new(false)),
        }
    }

    pub fn render(&mut self, cx: &ViewContext) -> AnyElement {
        self.subscriptions.clear();
        let dirty = self.dirty.clone();
        let (element, subs) =
            velox_reactive::track_render(|| self.component.render(cx), move || dirty.set(true));
        self.subscriptions = subs;
        element
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    pub fn clear_dirty(&self) {
        self.dirty.set(false);
    }

    pub fn component(&self) -> &C {
        &self.component
    }

    pub fn component_mut(&mut self) -> &mut C {
        &mut self.component
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{
        AnyElement, Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
    };
    use crate::parent::IntoAnyElement;
    use crate::style::Style;

    struct StubElement {
        style: Style,
    }

    impl StubElement {
        fn new() -> Self {
            Self {
                style: Style::new(),
            }
        }
    }

    #[derive(Default)]
    struct StubState;

    impl Element for StubElement {
        type State = StubState;

        fn layout(
            &mut self,
            _: &mut StubState,
            _: &[AnyElement],
            _: &mut LayoutContext,
        ) -> LayoutRequest {
            LayoutRequest {
                taffy_style: taffy::Style::default(),
            }
        }

        fn paint(&mut self, _: &mut StubState, _: velox_scene::Rect, _: &mut PaintContext) {}
    }

    impl HasStyle for StubElement {
        fn get_style(&self) -> &Style {
            &self.style
        }
    }

    impl IntoElement for StubElement {
        type Element = StubElement;
        fn into_element(self) -> StubElement {
            self
        }
    }

    impl IntoAnyElement for StubElement {
        fn into_any_element(self) -> AnyElement {
            AnyElement::new(self, None, vec![])
        }
    }

    struct Counter {
        count: velox_reactive::Signal<u32>,
    }

    impl Component for Counter {
        fn render(&self, _cx: &ViewContext) -> AnyElement {
            let _val = self.count.get();
            let child = StubElement::new().into_any_element();
            AnyElement::new(StubElement::new(), None, vec![child])
        }
    }

    #[test]
    fn component_renders_element_tree() {
        let counter = Counter {
            count: velox_reactive::Signal::new(42),
        };
        let mut host = ComponentHost::new(counter);
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);
        let element = host.render(&cx);
        assert!(!element.children().is_empty());
    }

    #[test]
    fn component_host_provides_access() {
        let counter = Counter {
            count: velox_reactive::Signal::new(0),
        };
        let mut host = ComponentHost::new(counter);
        host.component_mut().count.set(10);
        assert_eq!(host.component().count.get(), 10);
    }

    #[test]
    fn signal_change_marks_dirty() {
        let counter = Counter {
            count: velox_reactive::Signal::new(0),
        };
        let mut host = ComponentHost::new(counter);
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);

        host.render(&cx);
        assert!(!host.is_dirty());

        host.component().count.set(5);
        assert!(host.is_dirty());
    }

    #[test]
    fn clear_dirty_resets_flag() {
        let counter = Counter {
            count: velox_reactive::Signal::new(0),
        };
        let mut host = ComponentHost::new(counter);
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);

        host.render(&cx);
        host.component().count.set(5);
        assert!(host.is_dirty());

        host.clear_dirty();
        assert!(!host.is_dirty());
    }

    #[test]
    fn re_render_resubscribes() {
        let signal_a = velox_reactive::Signal::new(0u32);
        let signal_b = velox_reactive::Signal::new(0u32);

        struct TwoSignals {
            a: velox_reactive::Signal<u32>,
            b: velox_reactive::Signal<u32>,
            use_b: std::cell::Cell<bool>,
        }

        impl Component for TwoSignals {
            fn render(&self, _cx: &ViewContext) -> AnyElement {
                let _va = self.a.get();
                if self.use_b.get() {
                    let _vb = self.b.get();
                }
                StubElement::new().into_any_element()
            }
        }

        let comp = TwoSignals {
            a: signal_a.clone(),
            b: signal_b.clone(),
            use_b: std::cell::Cell::new(true),
        };
        let mut host = ComponentHost::new(comp);
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);

        host.render(&cx);
        signal_b.set(1);
        assert!(host.is_dirty());

        host.clear_dirty();
        host.component().use_b.set(false);
        host.render(&cx);

        signal_b.set(2);
        assert!(!host.is_dirty());
    }

    #[test]
    fn drop_host_cancels_subscriptions() {
        let signal = velox_reactive::Signal::new(0u32);

        struct Simple {
            s: velox_reactive::Signal<u32>,
        }
        impl Component for Simple {
            fn render(&self, _cx: &ViewContext) -> AnyElement {
                let _v = self.s.get();
                StubElement::new().into_any_element()
            }
        }

        let mut host = ComponentHost::new(Simple { s: signal.clone() });
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);
        host.render(&cx);

        drop(host);
        signal.set(99);
    }
}
