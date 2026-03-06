use crate::config::WindowConfig;
use crate::window_id::WindowId;

pub struct ManagedWindow {
    id: WindowId,
    window: winit::window::Window,
    config: WindowConfig,
}

impl ManagedWindow {
    pub fn new(window: winit::window::Window, config: WindowConfig) -> Self {
        let id = WindowId::from_winit(window.id());
        Self { id, window, config }
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn winit_window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn title(&self) -> &str {
        &self.config.title
    }
}
