use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_runtime::Runtime;
use velox_window::{WindowConfig, WindowManager};

pub(crate) struct VeloxHandler {
    runtime: Runtime,
    window_manager: WindowManager,
    pending_windows: Vec<WindowConfig>,
    initialized: bool,
}

impl VeloxHandler {
    pub(crate) fn new(runtime: Runtime, window_configs: Vec<WindowConfig>) -> Self {
        Self {
            runtime,
            window_manager: WindowManager::new(),
            pending_windows: window_configs,
            initialized: false,
        }
    }
}

impl ApplicationHandler for VeloxHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;

        let configs: Vec<WindowConfig> = self.pending_windows.drain(..).collect();
        if configs.is_empty() {
            event_loop.exit();
            return;
        }

        for config in configs {
            let _ = self.window_manager.create_window(event_loop, config);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.window_manager.close_by_winit_id(window_id);
                if self.window_manager.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(_) => {}
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.runtime.flush();
        self.window_manager.request_redraws();
    }
}
