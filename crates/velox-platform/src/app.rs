pub trait PlatformApp {
    fn hide(&self);
    fn show(&self);
    fn set_badge(&self, text: Option<&str>);
}
