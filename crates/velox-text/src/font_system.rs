use std::path::Path;

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

    pub fn add_fallback_font(&mut self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let data = std::fs::read(path.as_ref())?;
        self.inner.db_mut().load_font_data(data);
        Ok(())
    }

    pub fn load_system_fonts(&mut self) {
        self.inner.db_mut().load_system_fonts();
    }
}

impl Default for FontSystem {
    fn default() -> Self {
        Self::new()
    }
}
