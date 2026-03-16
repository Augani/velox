use std::cell::RefCell;

pub trait PlatformClipboard {
    fn read_text(&self) -> Option<String>;
    fn write_text(&self, text: &str);
}

pub struct NativeClipboard {
    clipboard: RefCell<Option<arboard::Clipboard>>,
}

impl NativeClipboard {
    pub fn new() -> Self {
        Self {
            clipboard: RefCell::new(arboard::Clipboard::new().ok()),
        }
    }

    fn with_clipboard<R>(&self, f: impl FnOnce(&mut arboard::Clipboard) -> R) -> Option<R> {
        let mut clipboard = self.clipboard.borrow_mut();
        let clipboard = clipboard.as_mut()?;
        Some(f(clipboard))
    }
}

impl Default for NativeClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformClipboard for NativeClipboard {
    fn read_text(&self) -> Option<String> {
        self.with_clipboard(|clipboard| clipboard.get_text().ok())
            .flatten()
    }

    fn write_text(&self, text: &str) {
        let _ = self.with_clipboard(|clipboard| clipboard.set_text(text.to_owned()));
    }
}
