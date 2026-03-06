pub struct FontSystem {
    inner: cosmic_text::FontSystem,
}

impl FontSystem {
    pub fn new() -> Self {
        Self {
            inner: cosmic_text::FontSystem::new(),
        }
    }

    pub(crate) fn inner_mut(&mut self) -> &mut cosmic_text::FontSystem {
        &mut self.inner
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
