use std::collections::HashMap;

use winit::event_loop::ActiveEventLoop;

use crate::config::WindowConfig;
use crate::managed::ManagedWindow;
use crate::window_id::WindowId;

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
        let attrs = config.to_window_attributes();
        let window = event_loop.create_window(attrs)?;
        let managed = ManagedWindow::new(window, config);
        let id = managed.id();
        self.windows.insert(id, managed);
        Ok(id)
    }

    pub fn close_window(&mut self, id: WindowId) -> bool {
        self.windows.remove(&id).is_some()
    }

    pub fn get(&self, id: WindowId) -> Option<&ManagedWindow> {
        self.windows.get(&id)
    }

    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut ManagedWindow> {
        self.windows.get_mut(&id)
    }

    pub fn find_by_winit_id(&self, winit_id: winit::window::WindowId) -> Option<&ManagedWindow> {
        let id = WindowId::from_winit(winit_id);
        self.windows.get(&id)
    }

    pub fn find_by_winit_id_mut(
        &mut self,
        winit_id: winit::window::WindowId,
    ) -> Option<&mut ManagedWindow> {
        let id = WindowId::from_winit(winit_id);
        self.windows.get_mut(&id)
    }

    pub fn request_redraws(&self) {
        for window in self.windows.values() {
            window.request_redraw();
        }
    }

    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    pub fn window_ids(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.windows.keys().copied()
    }

    pub fn windows(&self) -> impl Iterator<Item = &ManagedWindow> {
        self.windows.values()
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
