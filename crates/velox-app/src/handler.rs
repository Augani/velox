use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_runtime::Runtime;
use velox_scene::Scene;
use velox_window::{WindowConfig, WindowId, WindowManager};

pub(crate) struct VeloxHandler {
    runtime: Runtime,
    window_manager: WindowManager,
    pending_windows: Vec<WindowConfig>,
    initialized: bool,
    scenes: HashMap<WindowId, Scene>,
}

impl VeloxHandler {
    pub(crate) fn new(runtime: Runtime, window_configs: Vec<WindowConfig>) -> Self {
        Self {
            runtime,
            window_manager: WindowManager::new(),
            pending_windows: window_configs,
            initialized: false,
            scenes: HashMap::new(),
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
            if let Ok(window_id) = self.window_manager.create_window(event_loop, config) {
                self.scenes.insert(window_id, Scene::new());
            }
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
                let velox_id = WindowId::from_winit(window_id);
                self.scenes.remove(&velox_id);
                self.window_manager.close_by_winit_id(window_id);
                if self.window_manager.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(_) => {}
            WindowEvent::RedrawRequested => {
                let velox_id = WindowId::from_winit(window_id);
                if let Some(scene) = self.scenes.get_mut(&velox_id) {
                    scene.layout();
                    scene.paint();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.runtime.flush();
        self.window_manager.request_redraws();
    }
}
