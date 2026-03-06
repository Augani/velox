use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_render::{GlyphAtlas, GpuContext, Renderer, WindowSurface};
use velox_runtime::Runtime;
use velox_scene::{Scene, ShortcutRegistry};
use velox_window::{WindowConfig, WindowId, WindowManager};

struct WindowState {
    scene: Scene,
    surface: WindowSurface,
}

pub(crate) struct VeloxHandler {
    runtime: Runtime,
    window_manager: WindowManager,
    windows: HashMap<WindowId, WindowState>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    glyph_atlas: GlyphAtlas,
    shortcuts: ShortcutRegistry,
    pending_windows: Vec<WindowConfig>,
    setup: Option<Box<dyn FnOnce(&mut Scene)>>,
    initialized: bool,
}

impl VeloxHandler {
    pub(crate) fn new(
        runtime: Runtime,
        window_configs: Vec<WindowConfig>,
        setup: Option<Box<dyn FnOnce(&mut Scene)>>,
    ) -> Self {
        Self {
            runtime,
            window_manager: WindowManager::new(),
            windows: HashMap::new(),
            gpu: None,
            renderer: None,
            glyph_atlas: GlyphAtlas::new(1024, 1024),
            shortcuts: ShortcutRegistry::new(),
            pending_windows: window_configs,
            setup,
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

        let first_config = configs[0].clone();
        let first_label = first_config.id_label().to_owned();
        let first_window_id = match self.window_manager.create_window(event_loop, first_config) {
            Ok(id) => id,
            Err(error) => {
                eprintln!("[velox] failed to create window `{first_label}`: {error}");
                event_loop.exit();
                return;
            }
        };

        let first_window_arc = self
            .window_manager
            .get_window(first_window_id)
            .expect("just created")
            .window_arc();

        let gpu = GpuContext::new(None);
        let first_surface = WindowSurface::new(&gpu, first_window_arc);
        let renderer = Renderer::new(&gpu, first_surface.format());

        let mut first_scene = Scene::new();
        if let Some(setup) = self.setup.take() {
            setup(&mut first_scene);
        }
        self.windows.insert(
            first_window_id,
            WindowState {
                scene: first_scene,
                surface: first_surface,
            },
        );

        for config in configs.into_iter().skip(1) {
            let label = config.id_label().to_owned();
            match self.window_manager.create_window(event_loop, config) {
                Ok(window_id) => {
                    let window_arc = self
                        .window_manager
                        .get_window(window_id)
                        .expect("just created")
                        .window_arc();
                    let surface = WindowSurface::new(&gpu, window_arc);
                    self.windows.insert(
                        window_id,
                        WindowState {
                            scene: Scene::new(),
                            surface,
                        },
                    );
                }
                Err(error) => {
                    eprintln!("[velox] failed to create window `{label}`: {error}");
                }
            }
        }

        self.gpu = Some(gpu);
        self.renderer = Some(renderer);

        if self.window_manager.is_empty() {
            eprintln!("[velox] no windows were created, exiting");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let velox_id = WindowId::from_winit(window_id);

        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&velox_id);
                self.window_manager.close_by_winit_id(window_id);
                if self.window_manager.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let (Some(gpu), Some(ws)) = (&self.gpu, self.windows.get_mut(&velox_id)) {
                    ws.surface.resize(gpu, size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(gpu) = self.gpu.as_ref() else {
                    return;
                };
                if let Some(ws) = self.windows.get_mut(&velox_id)
                    && let Some(renderer) = self.renderer.as_mut()
                {
                    ws.scene.layout();
                    ws.scene.paint();
                    if let Err(err) = renderer.render(
                        gpu,
                        &ws.surface,
                        ws.scene.commands(),
                        &mut self.glyph_atlas,
                    ) {
                        match err {
                            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                                ws.surface
                                    .resize(gpu, ws.surface.width(), ws.surface.height());
                            }
                            wgpu::SurfaceError::OutOfMemory => {
                                eprintln!("[velox] GPU out of memory");
                                event_loop.exit();
                            }
                            _ => {}
                        }
                    }
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
