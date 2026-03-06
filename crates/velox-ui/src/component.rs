use crate::element::AnyElement;
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

pub struct ComponentHost<C: Component> {
    component: C,
}

impl<C: Component> ComponentHost<C> {
    pub fn new(component: C) -> Self {
        Self { component }
    }

    pub fn render(&self, cx: &ViewContext) -> AnyElement {
        self.component.render(cx)
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
        Element, HasStyle, IntoElement, LayoutContext, LayoutRequest, PaintContext,
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
        count: u32,
    }

    impl Component for Counter {
        fn render(&self, _cx: &ViewContext) -> AnyElement {
            let child = StubElement::new().into_any_element();
            AnyElement::new(StubElement::new(), None, vec![child])
        }
    }

    #[test]
    fn component_renders_element_tree() {
        let counter = Counter { count: 42 };
        let host = ComponentHost::new(counter);
        let theme = Theme::light();
        let cx = ViewContext::new(&theme);
        let element = host.render(&cx);
        assert!(!element.children().is_empty());
    }

    #[test]
    fn component_host_provides_access() {
        let counter = Counter { count: 0 };
        let mut host = ComponentHost::new(counter);
        host.component_mut().count = 10;
        assert_eq!(host.component().count, 10);
    }
}
