use crate::range::ViewportRange;

#[derive(Default)]
pub struct ListCallbacks {
    pub on_visible_range_changed: Option<Box<dyn Fn(ViewportRange)>>,
    pub on_prefetch_range_changed: Option<Box<dyn Fn(ViewportRange)>>,
    pub on_item_visible: Option<Box<dyn Fn(usize)>>,
    pub on_item_hidden: Option<Box<dyn Fn(usize)>>,
}

impl ListCallbacks {
    pub fn with_on_visible_range_changed(
        mut self,
        callback: impl Fn(ViewportRange) + 'static,
    ) -> Self {
        self.on_visible_range_changed = Some(Box::new(callback));
        self
    }

    pub fn with_on_prefetch_range_changed(
        mut self,
        callback: impl Fn(ViewportRange) + 'static,
    ) -> Self {
        self.on_prefetch_range_changed = Some(Box::new(callback));
        self
    }

    pub fn with_on_item_visible(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_item_visible = Some(Box::new(callback));
        self
    }

    pub fn with_on_item_hidden(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_item_hidden = Some(Box::new(callback));
        self
    }
}
