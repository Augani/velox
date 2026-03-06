use crate::element::AnyElement;

pub trait ParentElement: Sized {
    fn children_mut(&mut self) -> &mut Vec<AnyElement>;

    fn child(mut self, child: impl IntoAnyElement) -> Self {
        let any = child.into_any_element();
        self.children_mut().push(any);
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = impl IntoAnyElement>) -> Self {
        for child in children {
            self.children_mut().push(child.into_any_element());
        }
        self
    }
}

pub trait IntoAnyElement {
    fn into_any_element(self) -> AnyElement;
}
