pub trait PlatformClipboard {
    fn read_text(&self) -> Option<String>;
    fn write_text(&self, text: &str);
}
