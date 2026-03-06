use std::collections::HashMap;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_animation::AnimationManager;
use velox_render::{GlyphAtlas, GpuContext, Renderer, WindowSurface};
use velox_runtime::Runtime;
use velox_scene::{Scene, ShortcutRegistry};
use velox_style::ThemeManager;
use velox_window::{WindowConfig, WindowId, WindowManager};

type SetupFn = Box<dyn FnOnce(&mut Scene)>;

struct WindowState {
    scene: Scene,
    surface: WindowSurface,
    needs_redraw: bool,
    scene_dirty: bool,
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
    setup: Option<SetupFn>,
    initialized: bool,
    current_modifiers: winit::keyboard::ModifiersState,
    cursor_position: velox_scene::Point,
    clipboard: Option<arboard::Clipboard>,
    theme_manager: Option<ThemeManager>,
    last_theme_version: Option<u64>,
    animation_manager: AnimationManager,
    last_frame_time: Option<Instant>,
}

impl VeloxHandler {
    pub(crate) fn new(
        runtime: Runtime,
        window_configs: Vec<WindowConfig>,
        setup: Option<SetupFn>,
        theme_manager: Option<ThemeManager>,
    ) -> Self {
        let last_theme_version = theme_manager.as_ref().map(ThemeManager::version);
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
            current_modifiers: winit::keyboard::ModifiersState::default(),
            cursor_position: velox_scene::Point::new(0.0, 0.0),
            clipboard: None,
            theme_manager,
            last_theme_version,
            animation_manager: AnimationManager::new(),
            last_frame_time: None,
        }
    }

    fn request_pending_redraws(&mut self) {
        let pending: Vec<WindowId> = self
            .windows
            .iter()
            .filter_map(|(id, ws)| ws.needs_redraw.then_some(*id))
            .collect();

        for id in pending {
            if let Some(ws) = self.windows.get_mut(&id) {
                ws.needs_redraw = false;
            }
            if let Some(managed) = self.window_manager.get_window(id) {
                managed.window().request_redraw();
            }
        }
    }

    fn clipboard_read_text(&mut self) -> Option<String> {
        self.clipboard.as_mut().and_then(|c| c.get_text().ok())
    }

    fn clipboard_write_text(&mut self, text: String) {
        if let Some(clipboard) = self.clipboard.as_mut() {
            let _ = clipboard.set_text(text);
        }
    }

    fn sync_theme_updates(&mut self) {
        let Some(theme_manager) = self.theme_manager.as_ref() else {
            return;
        };

        let version = theme_manager.version();
        if self.last_theme_version == Some(version) {
            return;
        }
        self.last_theme_version = Some(version);

        for ws in self.windows.values_mut() {
            ws.needs_redraw = true;
            ws.scene_dirty = true;
        }
    }
}

impl ApplicationHandler for VeloxHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        self.clipboard = arboard::Clipboard::new().ok();

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
                needs_redraw: true,
                scene_dirty: true,
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
                            needs_redraw: true,
                            scene_dirty: true,
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
                    ws.needs_redraw = true;
                    ws.scene_dirty = true;
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(gpu) = self.gpu.as_ref() else {
                    return;
                };
                if let Some(ws) = self.windows.get_mut(&velox_id)
                    && let Some(renderer) = self.renderer.as_mut()
                {
                    if ws.scene_dirty {
                        ws.scene.layout();
                        ws.scene.paint();
                        ws.scene_dirty = false;
                    }
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
                                ws.needs_redraw = true;
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
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.state.is_pressed() {
                    return;
                }
                let Some(velox_key) = crate::key_convert::convert_key(&event.logical_key) else {
                    return;
                };
                let modifiers = crate::key_convert::convert_modifiers(self.current_modifiers);

                if self.shortcuts.handle_key_event(velox_key, modifiers) {
                    return;
                }

                let clipboard_read = self.clipboard_read_text();
                let mut clipboard_write = None;
                if let Some(ws) = self.windows.get_mut(&velox_id)
                    && let Some(focused) = ws.scene.focus().focused()
                {
                    let text = event.text.as_ref().map(|t| t.to_string());
                    let key_event = velox_scene::KeyEvent {
                        key: velox_key,
                        modifiers,
                        state: crate::key_convert::convert_element_state(event.state),
                        text,
                    };
                    let result = ws
                        .scene
                        .tree_mut()
                        .dispatch_key_event_with_context(focused, &key_event, clipboard_read);
                    if result.redraw_requested {
                        ws.needs_redraw = true;
                        ws.scene_dirty = true;
                    }
                    clipboard_write = result.clipboard_write;
                }
                if let Some(text) = clipboard_write {
                    self.clipboard_write_text(text);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(managed) = self.window_manager.get_window_mut(velox_id) {
                    managed.set_scale_factor(scale_factor);
                }
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    ws.needs_redraw = true;
                    ws.scene_dirty = true;
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.current_modifiers = mods.state();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position =
                    velox_scene::Point::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == winit::event::ElementState::Pressed
                    && button == winit::event::MouseButton::Left
                {
                    let mut clipboard_write = None;
                    if let Some(ws) = self.windows.get_mut(&velox_id) {
                        let point = self.cursor_position;
                        if let Some(hit_id) = ws.scene.hit_test(point) {
                            let was_focused = ws.scene.focus().focused();
                            ws.scene.focus_mut().request_focus(hit_id);
                            if was_focused != Some(hit_id) {
                                ws.needs_redraw = true;
                                ws.scene_dirty = true;
                            }
                            let node_rect = ws
                                .scene
                                .tree()
                                .rect(hit_id)
                                .unwrap_or(velox_scene::Rect::zero());
                            let local_pos = velox_scene::Point::new(
                                point.x - node_rect.x,
                                point.y - node_rect.y,
                            );
                            let mouse_event = velox_scene::MouseEvent {
                                position: local_pos,
                                button: velox_scene::MouseButton::Left,
                                state: velox_scene::ButtonState::Pressed,
                                click_count: 1,
                                modifiers: crate::key_convert::convert_modifiers(
                                    self.current_modifiers,
                                ),
                            };
                            let result = ws
                                .scene
                                .tree_mut()
                                .dispatch_mouse_event_with_context(hit_id, &mouse_event);
                            if result.redraw_requested {
                                ws.needs_redraw = true;
                                ws.scene_dirty = true;
                            }
                            clipboard_write = result.clipboard_write;
                        }
                    }
                    if let Some(text) = clipboard_write {
                        self.clipboard_write_text(text);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        (pos.x as f32, pos.y as f32)
                    }
                };
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    let point = self.cursor_position;
                    if let Some(hit_id) = ws.scene.hit_test(point) {
                        let scroll_event = velox_scene::ScrollEvent {
                            delta_x,
                            delta_y,
                            modifiers: crate::key_convert::convert_modifiers(
                                self.current_modifiers,
                            ),
                        };
                        let result = ws
                            .scene
                            .tree_mut()
                            .dispatch_scroll_event_with_context(hit_id, &scroll_event);
                        if result.redraw_requested {
                            ws.needs_redraw = true;
                            ws.scene_dirty = true;
                        }
                    }
                }
            }
            WindowEvent::Ime(ime) => {
                let Some(ws) = self.windows.get_mut(&velox_id) else {
                    return;
                };
                let Some(focused) = ws.scene.focus().focused() else {
                    return;
                };
                let ime_event = match ime {
                    winit::event::Ime::Enabled => velox_scene::ImeEvent::Enabled,
                    winit::event::Ime::Disabled => velox_scene::ImeEvent::Disabled,
                    winit::event::Ime::Preedit(text, cursor) => velox_scene::ImeEvent::Preedit {
                        text,
                        cursor_range: cursor,
                    },
                    winit::event::Ime::Commit(text) => velox_scene::ImeEvent::Commit { text },
                };
                let result = ws
                    .scene
                    .tree_mut()
                    .dispatch_ime_event_with_context(focused, &ime_event);
                if result.redraw_requested {
                    ws.needs_redraw = true;
                    ws.scene_dirty = true;
                }
            }
            WindowEvent::DroppedFile(path) => {
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    let point = self.cursor_position;
                    let payload = velox_scene::DragPayload::Files(vec![path]);
                    if let Some(target_id) = ws.scene.tree().find_drop_target(point) {
                        ws.scene.tree_mut().dispatch_drop(target_id, payload, point);
                        ws.needs_redraw = true;
                        ws.scene_dirty = true;
                    }
                }
            }
            WindowEvent::HoveredFile(_path) => {}
            WindowEvent::HoveredFileCancelled => {}
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.runtime.flush();

        let now = Instant::now();
        let dt = self
            .last_frame_time
            .map(|last| now.duration_since(last))
            .unwrap_or_default();
        self.last_frame_time = Some(now);

        let policy = self.runtime.power_policy();
        self.animation_manager.tick(dt, policy);

        if self.animation_manager.has_running() {
            for ws in self.windows.values_mut() {
                ws.needs_redraw = true;
                ws.scene_dirty = true;
            }
        }

        let no_redraws = !self.windows.values().any(|ws| ws.needs_redraw);
        if no_redraws && self.runtime.has_pending_idle() {
            self.runtime.flush_idle();
        }

        self.sync_theme_updates();
        self.request_pending_redraws();
    }
}
