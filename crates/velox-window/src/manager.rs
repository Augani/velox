use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;

use crate::config::WindowConfig;
use crate::window_id::WindowId;

pub struct ManagedWindow {
    window: winit::window::Window,
    label: String,
}

impl ManagedWindow {
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

pub struct WindowManager {
    windows: HashMap<WindowId, ManagedWindow>,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
        }
    }

    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        config: WindowConfig,
    ) -> Result<WindowId, winit::error::OsError> {
        let label = config.id_label().to_owned();
        let attrs = config.to_window_attributes();
        let window = event_loop.create_window(attrs)?;
        let id = WindowId::from_winit(window.id());
        self.windows.insert(id, ManagedWindow { window, label });
        Ok(id)
    }

    pub fn close_window(&mut self, id: WindowId) -> bool {
        self.windows.remove(&id).is_some()
    }

    pub fn get_window(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.windows.get(&id)
    }

    pub fn find_by_winit_id(&self, winit_id: winit::window::WindowId) -> Option<&ManagedWindow> {
        let id = WindowId::from_winit(winit_id);
        self.windows.get(&id)
    }

    pub fn close_by_winit_id(&mut self, winit_id: winit::window::WindowId) -> bool {
        let id = WindowId::from_winit(winit_id);
        self.windows.remove(&id).is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn request_redraws(&self) {
        for managed in self.windows.values() {
            managed.window.request_redraw();
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
