use std::collections::HashMap;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use velox_animation::AnimationManager;
use velox_platform::{AccessibilityActionKind, PlatformClipboard, WindowAccessibilityAdapter};
use velox_render::{GlyphAtlas, GpuContext, Renderer, WindowSurface};
use velox_runtime::Runtime;
use velox_scene::{Scene, ShortcutRegistry};
use velox_style::ThemeManager;
use velox_text::{FontSystem, GlyphRasterizer};
use velox_ui::UiRoot;
use velox_ui::element::PaintContext;
use velox_window::{WindowConfig, WindowId, WindowManager};

type SetupFn = Box<dyn FnMut(&mut Scene)>;
pub(crate) type UiRenderFn = Box<dyn FnMut() -> Vec<velox_ui::element::AnyElement>>;

struct WindowState {
    scene: Scene,
    surface: WindowSurface,
    accessibility: WindowAccessibilityAdapter,
    scale_factor: f64,
    needs_redraw: bool,
    scene_dirty: bool,
    cursor_position: velox_scene::Point,
    ui_root: Option<UiRoot>,
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
    app_name: String,
    current_modifiers: winit::keyboard::ModifiersState,
    clipboard: Option<Box<dyn PlatformClipboard>>,
    theme_manager: Option<ThemeManager>,
    last_theme_version: Option<u64>,
    animation_manager: AnimationManager,
    last_frame_time: Option<Instant>,
    continuous_redraw: bool,
    ui_render: Option<UiRenderFn>,
    font_system: FontSystem,
    glyph_rasterizer: GlyphRasterizer,
}

impl VeloxHandler {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        runtime: Runtime,
        app_name: String,
        window_configs: Vec<WindowConfig>,
        setup: Option<SetupFn>,
        ui_render: Option<UiRenderFn>,
        theme_manager: Option<ThemeManager>,
        continuous_redraw: bool,
        clipboard: Option<Box<dyn PlatformClipboard>>,
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
            app_name,
            current_modifiers: winit::keyboard::ModifiersState::default(),
            clipboard,
            theme_manager,
            last_theme_version,
            animation_manager: AnimationManager::new(),
            last_frame_time: None,
            continuous_redraw,
            ui_render,
            font_system: FontSystem::new(),
            glyph_rasterizer: GlyphRasterizer::new(),
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
        self.clipboard
            .as_ref()
            .and_then(|clipboard| clipboard.read_text())
    }

    fn clipboard_write_text(&mut self, text: String) {
        if let Some(clipboard) = self.clipboard.as_ref() {
            clipboard.write_text(&text);
        }
    }

    fn set_window_cursor(&self, velox_id: WindowId, cursor: velox_ui::CursorStyle) {
        let winit_cursor = match cursor {
            velox_ui::CursorStyle::Default => winit::window::CursorIcon::Default,
            velox_ui::CursorStyle::Pointer => winit::window::CursorIcon::Pointer,
            velox_ui::CursorStyle::Text => winit::window::CursorIcon::Text,
            velox_ui::CursorStyle::Grab => winit::window::CursorIcon::Grab,
            velox_ui::CursorStyle::Grabbing => winit::window::CursorIcon::Grabbing,
            velox_ui::CursorStyle::NotAllowed => winit::window::CursorIcon::NotAllowed,
            velox_ui::CursorStyle::Move => winit::window::CursorIcon::Move,
            velox_ui::CursorStyle::Crosshair => winit::window::CursorIcon::Crosshair,
            velox_ui::CursorStyle::Wait => winit::window::CursorIcon::Wait,
            velox_ui::CursorStyle::Progress => winit::window::CursorIcon::Progress,
            velox_ui::CursorStyle::Help => winit::window::CursorIcon::Help,
            velox_ui::CursorStyle::ZoomIn => winit::window::CursorIcon::ZoomIn,
            velox_ui::CursorStyle::ZoomOut => winit::window::CursorIcon::ZoomOut,
            velox_ui::CursorStyle::ResizeN => winit::window::CursorIcon::NResize,
            velox_ui::CursorStyle::ResizeS => winit::window::CursorIcon::SResize,
            velox_ui::CursorStyle::ResizeE => winit::window::CursorIcon::EResize,
            velox_ui::CursorStyle::ResizeW => winit::window::CursorIcon::WResize,
            velox_ui::CursorStyle::ResizeNE => winit::window::CursorIcon::NeResize,
            velox_ui::CursorStyle::ResizeNW => winit::window::CursorIcon::NwResize,
            velox_ui::CursorStyle::ResizeSE => winit::window::CursorIcon::SeResize,
            velox_ui::CursorStyle::ResizeSW => winit::window::CursorIcon::SwResize,
            velox_ui::CursorStyle::ResizeEW => winit::window::CursorIcon::EwResize,
            velox_ui::CursorStyle::ResizeNS => winit::window::CursorIcon::NsResize,
        };
        if let Some(managed) = self.window_manager.get_window(velox_id) {
            managed.window().set_cursor(winit_cursor);
        }
    }

    fn flush_accessibility_actions(&mut self) {
        let mut clipboard_writes = Vec::new();
        for ws in self.windows.values_mut() {
            let actions = ws.accessibility.drain_pending_actions();
            if actions.is_empty() {
                continue;
            }

            let mut needs_redraw = false;
            for action in actions {
                if let Some(ui_root) = ws.ui_root.as_mut() {
                    let action_redraw = match action.kind {
                        AccessibilityActionKind::Focus => {
                            ui_root.request_accessibility_focus(action.target)
                        }
                        AccessibilityActionKind::Blur => {
                            ui_root.clear_accessibility_focus(action.target)
                        }
                        AccessibilityActionKind::Click => {
                            ui_root.activate_accessibility(action.target)
                        }
                        AccessibilityActionKind::SetValue => {
                            action.text.clone().is_some_and(|text| {
                                ui_root.set_accessibility_value(action.target, text)
                            })
                        }
                        AccessibilityActionKind::ReplaceSelectedText => {
                            action.text.clone().is_some_and(|text| {
                                ui_root.replace_accessibility_selected_text(action.target, text)
                            })
                        }
                        AccessibilityActionKind::SetTextSelection => {
                            action.selection.is_some_and(|selection| {
                                ui_root.set_accessibility_text_selection(action.target, selection)
                            })
                        }
                        AccessibilityActionKind::Other => false,
                    };
                    needs_redraw |= action_redraw;
                } else {
                    let action_result = match action.kind {
                        AccessibilityActionKind::Focus => velox_scene::EventDispatchResult {
                            redraw_requested: ws.scene.request_focus(action.target),
                            ..velox_scene::EventDispatchResult::default()
                        },
                        AccessibilityActionKind::Blur => velox_scene::EventDispatchResult {
                            redraw_requested: ws.scene.blur_accessibility(action.target),
                            ..velox_scene::EventDispatchResult::default()
                        },
                        AccessibilityActionKind::Click => {
                            ws.scene.activate_accessibility(action.target)
                        }
                        AccessibilityActionKind::SetValue => action.text.as_ref().map_or(
                            velox_scene::EventDispatchResult::default(),
                            |text| {
                                ws.scene.handle_accessibility_action(
                                    action.target,
                                    &velox_scene::AccessibilityAction::SetValue(text.clone()),
                                )
                            },
                        ),
                        AccessibilityActionKind::ReplaceSelectedText => action
                            .text
                            .as_ref()
                            .map_or(velox_scene::EventDispatchResult::default(), |text| {
                                ws.scene.handle_accessibility_action(
                                    action.target,
                                    &velox_scene::AccessibilityAction::ReplaceSelectedText(
                                        text.clone(),
                                    ),
                                )
                            }),
                        AccessibilityActionKind::SetTextSelection => action.selection.map_or(
                            velox_scene::EventDispatchResult::default(),
                            |selection| {
                                ws.scene.handle_accessibility_action(
                                    action.target,
                                    &velox_scene::AccessibilityAction::SetTextSelection(selection),
                                )
                            },
                        ),
                        AccessibilityActionKind::Other => {
                            velox_scene::EventDispatchResult::default()
                        }
                    };
                    needs_redraw |= action_result.redraw_requested;
                    if let Some(text) = action_result.clipboard_write {
                        clipboard_writes.push(text);
                    }
                }
            }

            if needs_redraw {
                ws.needs_redraw = true;
                ws.scene_dirty = true;
            }
        }

        for text in clipboard_writes {
            self.clipboard_write_text(text);
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

    fn build_window_contents(
        &mut self,
        logical_width: f32,
        logical_height: f32,
    ) -> (Scene, Option<UiRoot>) {
        let mut scene = Scene::new();
        if let Some(setup) = self.setup.as_mut() {
            setup(&mut scene);
        }

        let ui_root = if let Some(render_fn) = self.ui_render.as_mut() {
            let elements = render_fn();
            let mut ui_root = UiRoot::new();
            ui_root.set_root(elements, &mut self.font_system);
            ui_root.layout(logical_width, logical_height);
            Some(ui_root)
        } else {
            None
        };

        if let Some(root) = scene.tree().root() {
            scene.tree_mut().set_rect(
                root,
                velox_scene::Rect::new(0.0, 0.0, logical_width, logical_height),
            );
        }

        (scene, ui_root)
    }

    fn build_window_state(
        &mut self,
        surface: WindowSurface,
        accessibility: WindowAccessibilityAdapter,
        scale_factor: f64,
        logical_width: f32,
        logical_height: f32,
    ) -> WindowState {
        let (scene, mut ui_root) = self.build_window_contents(logical_width, logical_height);
        let initial_snapshot = if let Some(ui_root) = ui_root.as_mut() {
            ui_root.build_accessibility_tree()
        } else {
            let focused = scene.focus().focused();
            scene.tree().build_accessibility_tree(focused)
        };
        let mut state = WindowState {
            scene,
            surface,
            accessibility,
            scale_factor,
            needs_redraw: true,
            scene_dirty: true,
            cursor_position: velox_scene::Point::new(0.0, 0.0),
            ui_root,
        };
        state.accessibility.update(initial_snapshot);
        state
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

        let first_managed = self
            .window_manager
            .get_window(first_window_id)
            .expect("just created");
        let first_scale_factor = first_managed.scale_factor();
        let first_window_arc = first_managed.window_arc();
        let first_accessibility =
            WindowAccessibilityAdapter::new(first_window_arc.as_ref(), Some(self.app_name.clone()));

        let gpu = GpuContext::new(None);
        let first_surface = WindowSurface::new(&gpu, first_window_arc);
        let renderer = Renderer::new(&gpu, first_surface.format());

        let logical_w = first_surface.width() as f64 / first_scale_factor;
        let logical_h = first_surface.height() as f64 / first_scale_factor;

        let first_state = self.build_window_state(
            first_surface,
            first_accessibility,
            first_scale_factor,
            logical_w as f32,
            logical_h as f32,
        );
        self.windows.insert(first_window_id, first_state);
        if let Some(managed) = self.window_manager.get_window(first_window_id) {
            managed.window().set_visible(true);
        }

        for config in configs.into_iter().skip(1) {
            let label = config.id_label().to_owned();
            match self.window_manager.create_window(event_loop, config) {
                Ok(window_id) => {
                    let managed = self
                        .window_manager
                        .get_window(window_id)
                        .expect("just created");
                    let sf = managed.scale_factor();
                    let window_arc = managed.window_arc();
                    let accessibility = WindowAccessibilityAdapter::new(
                        window_arc.as_ref(),
                        Some(self.app_name.clone()),
                    );
                    let surface = WindowSurface::new(&gpu, window_arc);
                    let logical_w = surface.width() as f32 / sf as f32;
                    let logical_h = surface.height() as f32 / sf as f32;
                    let state =
                        self.build_window_state(surface, accessibility, sf, logical_w, logical_h);
                    self.windows.insert(window_id, state);
                    if let Some(managed) = self.window_manager.get_window(window_id) {
                        managed.window().set_visible(true);
                    }
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
        let window_arc = self
            .window_manager
            .get_window(velox_id)
            .map(|managed| managed.window_arc());
        if let (Some(window), Some(ws)) = (window_arc.as_ref(), self.windows.get_mut(&velox_id)) {
            ws.accessibility.process_event(window.as_ref(), &event);
        }

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
                    let logical_w = size.width as f64 / ws.scale_factor;
                    let logical_h = size.height as f64 / ws.scale_factor;
                    if let Some(root) = ws.scene.tree().root() {
                        ws.scene.tree_mut().set_rect(
                            root,
                            velox_scene::Rect::new(0.0, 0.0, logical_w as f32, logical_h as f32),
                        );
                    }
                    if let Some(ui_root) = ws.ui_root.as_mut() {
                        ui_root.mark_needs_layout();
                    }
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
                        let sf = ws.scale_factor as f32;
                        ws.scene.commands_mut().set_scale_factor(sf);
                        let logical_w = ws.surface.width() as f32 / sf;
                        let logical_h = ws.surface.height() as f32 / sf;

                        if let (Some(ui_root), Some(render_fn)) =
                            (ws.ui_root.as_mut(), self.ui_render.as_mut())
                        {
                            let elements = render_fn();
                            ui_root.set_root(elements, &mut self.font_system);
                            ui_root.layout(logical_w, logical_h);

                            let new_hit = ui_root.hit_test(ws.cursor_position);
                            ui_root
                                .coordinator_mut()
                                .handle_mouse_move(new_hit, ws.cursor_position);

                            let theme = self
                                .theme_manager
                                .as_ref()
                                .map(|tm| tm.current().clone())
                                .unwrap_or_else(velox_style::Theme::light);
                            let commands = ws.scene.commands_mut();
                            commands.clear();
                            let hovered = ui_root.coordinator().hovered_node();
                            let active = ui_root.coordinator().active_node();
                            let focused = ui_root.coordinator().focused_node();
                            let mut cx = PaintContext::new(
                                commands,
                                &theme,
                                &mut self.font_system,
                                &mut self.glyph_rasterizer,
                            )
                            .with_hovered(hovered)
                            .with_active(active)
                            .with_focused(focused)
                            .with_scale_factor(sf);
                            ui_root.paint(&mut cx);
                            let snapshot = ui_root.build_accessibility_tree();
                            ws.accessibility.update(snapshot);
                        } else {
                            ws.scene.layout();
                            if self.continuous_redraw {
                                ws.scene.paint_uncached();
                            } else {
                                ws.scene.paint();
                            }
                            let focused = ws.scene.focus().focused();
                            let snapshot = ws.scene.tree().build_accessibility_tree(focused);
                            ws.accessibility.update(snapshot);
                        }
                        ws.scene_dirty = false;
                    }
                    if let Err(err) = renderer.render(
                        gpu,
                        &ws.surface,
                        ws.scene.commands(),
                        &mut self.glyph_atlas,
                        ws.scale_factor as f32,
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
                    } else if self.continuous_redraw {
                        ws.needs_redraw = true;
                        ws.scene_dirty = true;
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
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    if let Some(ui_root) = ws.ui_root.as_mut() {
                        if velox_key == velox_scene::Key::Tab {
                            let shift = modifiers.contains(velox_scene::Modifiers::SHIFT);
                            let result = ui_root.coordinator_mut().handle_tab(shift);
                            if result.needs_redraw {
                                ws.needs_redraw = true;
                                ws.scene_dirty = true;
                            }
                            return;
                        }

                        let text = event.text.as_ref().map(|t| t.to_string());
                        let key_event = velox_scene::KeyEvent {
                            key: velox_key,
                            modifiers,
                            state: crate::key_convert::convert_element_state(event.state),
                            text,
                        };
                        let result = ui_root.coordinator_mut().handle_key_down(&key_event);
                        if result.needs_redraw {
                            ws.needs_redraw = true;
                            ws.scene_dirty = true;
                        }
                    } else if let Some(focused) = ws.scene.focus().focused() {
                        let text = event.text.as_ref().map(|t| t.to_string());
                        let key_event = velox_scene::KeyEvent {
                            key: velox_key,
                            modifiers,
                            state: crate::key_convert::convert_element_state(event.state),
                            text,
                        };
                        let result = ws.scene.tree_mut().dispatch_key_event_with_context(
                            focused,
                            &key_event,
                            clipboard_read,
                        );
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
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(managed) = self.window_manager.get_window_mut(velox_id) {
                    managed.set_scale_factor(scale_factor);
                }
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    ws.scale_factor = scale_factor;
                    let logical_w = ws.surface.width() as f64 / scale_factor;
                    let logical_h = ws.surface.height() as f64 / scale_factor;
                    if let Some(root) = ws.scene.tree().root() {
                        ws.scene.tree_mut().set_rect(
                            root,
                            velox_scene::Rect::new(0.0, 0.0, logical_w as f32, logical_h as f32),
                        );
                    }
                    if let Some(ui_root) = ws.ui_root.as_mut() {
                        ui_root.mark_needs_layout();
                    }
                    ws.needs_redraw = true;
                    ws.scene_dirty = true;
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.current_modifiers = mods.state();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mut cursor_changed = None;
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    let sf = ws.scale_factor as f32;
                    ws.cursor_position =
                        velox_scene::Point::new(position.x as f32 / sf, position.y as f32 / sf);

                    if let Some(ui_root) = ws.ui_root.as_mut() {
                        let hit = ui_root.hit_test(ws.cursor_position);
                        let result = ui_root
                            .coordinator_mut()
                            .handle_mouse_move(hit, ws.cursor_position);
                        if result.needs_redraw {
                            ws.needs_redraw = true;
                            ws.scene_dirty = true;
                        }
                        cursor_changed = result.cursor_changed;
                    }
                }
                if let Some(cursor) = cursor_changed {
                    self.set_window_cursor(velox_id, cursor);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let velox_button = match button {
                    winit::event::MouseButton::Left => velox_scene::MouseButton::Left,
                    winit::event::MouseButton::Right => velox_scene::MouseButton::Right,
                    winit::event::MouseButton::Middle => velox_scene::MouseButton::Middle,
                    _ => velox_scene::MouseButton::Left,
                };
                let mut cursor_changed = None;
                let mut clipboard_write = None;
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    let point = ws.cursor_position;
                    if let Some(ui_root) = ws.ui_root.as_mut() {
                        let hit = ui_root.hit_test(point);
                        let result = if state == winit::event::ElementState::Pressed {
                            if let Some(hit_id) = hit {
                                ui_root.coordinator_mut().handle_mouse_down(
                                    hit_id,
                                    point,
                                    velox_button,
                                )
                            } else {
                                velox_ui::EventResult {
                                    needs_redraw: false,
                                    cursor_changed: None,
                                }
                            }
                        } else {
                            ui_root
                                .coordinator_mut()
                                .handle_mouse_up(hit, point, velox_button)
                        };

                        if result.needs_redraw {
                            ws.needs_redraw = true;
                            ws.scene_dirty = true;
                        }
                        cursor_changed = result.cursor_changed;
                    } else if state == winit::event::ElementState::Pressed
                        && button == winit::event::MouseButton::Left
                        && let Some(hit_id) = ws.scene.hit_test(point)
                    {
                        if ws.scene.request_focus(hit_id) {
                            ws.needs_redraw = true;
                            ws.scene_dirty = true;
                        }
                        let node_rect = ws
                            .scene
                            .tree()
                            .rect(hit_id)
                            .unwrap_or(velox_scene::Rect::zero());
                        let local_pos =
                            velox_scene::Point::new(point.x - node_rect.x, point.y - node_rect.y);
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
                if let Some(cursor) = cursor_changed {
                    self.set_window_cursor(velox_id, cursor);
                }
                if let Some(text) = clipboard_write {
                    self.clipboard_write_text(text);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x * 40.0, y * 40.0),
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                let scroll_event = velox_scene::ScrollEvent {
                    delta_x,
                    delta_y,
                    modifiers: crate::key_convert::convert_modifiers(self.current_modifiers),
                };
                if let Some(ws) = self.windows.get_mut(&velox_id) {
                    let point = ws.cursor_position;
                    if let Some(ui_root) = ws.ui_root.as_mut() {
                        let hit = ui_root.hit_test(point);
                        if let Some(hit_id) = hit {
                            let result = ui_root
                                .coordinator_mut()
                                .handle_scroll(hit_id, &scroll_event);
                            if result.needs_redraw {
                                ws.needs_redraw = true;
                                ws.scene_dirty = true;
                            }
                        }
                    } else if let Some(hit_id) = ws.scene.hit_test(point) {
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
                    let point = ws.cursor_position;
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
        self.flush_accessibility_actions();

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

        let dt_secs = dt.as_secs_f32();
        for ws in self.windows.values_mut() {
            let coord_tick = ws
                .ui_root
                .as_mut()
                .map(|ui_root| ui_root.coordinator_mut().tick(dt_secs))
                .unwrap_or(false);
            if coord_tick {
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use velox_scene::Rect;
    use velox_ui::parent::IntoAnyElement;
    use velox_ui::{Styled, div, px};

    #[test]
    fn build_window_contents_runs_setup_for_each_window() {
        let setup_calls = Rc::new(Cell::new(0usize));
        let setup_counter = setup_calls.clone();
        let mut handler = VeloxHandler::new(
            Runtime::new(),
            String::from("Test App"),
            vec![],
            Some(Box::new(move |scene| {
                setup_counter.set(setup_counter.get() + 1);
                let root = scene.tree_mut().insert(None);
                scene
                    .tree_mut()
                    .set_rect(root, Rect::new(0.0, 0.0, 1.0, 1.0));
            })),
            None,
            None,
            false,
            None,
        );

        let (scene_a, ui_a) = handler.build_window_contents(800.0, 600.0);
        let (scene_b, ui_b) = handler.build_window_contents(320.0, 240.0);

        assert!(ui_a.is_none());
        assert!(ui_b.is_none());
        assert_eq!(setup_calls.get(), 2);

        let root_a = scene_a
            .tree()
            .root()
            .expect("first scene should have a root");
        let root_b = scene_b
            .tree()
            .root()
            .expect("second scene should have a root");

        assert_eq!(
            scene_a.tree().rect(root_a),
            Some(Rect::new(0.0, 0.0, 800.0, 600.0))
        );
        assert_eq!(
            scene_b.tree().rect(root_b),
            Some(Rect::new(0.0, 0.0, 320.0, 240.0))
        );
    }

    #[test]
    fn build_window_contents_creates_independent_ui_roots() {
        let render_calls = Rc::new(Cell::new(0usize));
        let render_counter = render_calls.clone();
        let mut handler = VeloxHandler::new(
            Runtime::new(),
            String::from("Test App"),
            vec![],
            None,
            Some(Box::new(move || {
                render_counter.set(render_counter.get() + 1);
                vec![div().w(px(40.0)).h(px(20.0)).into_any_element()]
            })),
            None,
            false,
            None,
        );

        let (_, mut ui_a) = handler.build_window_contents(400.0, 300.0);
        let (_, ui_b) = handler.build_window_contents(200.0, 100.0);

        assert_eq!(render_calls.get(), 2);

        let ui_a = ui_a.as_mut().expect("first window should have a ui root");
        let ui_b = ui_b.as_ref().expect("second window should have a ui root");

        assert!(!ui_a.needs_layout());
        assert!(!ui_b.needs_layout());

        ui_a.mark_needs_layout();

        assert!(ui_a.needs_layout());
        assert!(!ui_b.needs_layout());
    }

    #[test]
    fn clipboard_hooks_use_platform_trait() {
        #[derive(Clone)]
        struct TestClipboard {
            read_value: Rc<std::cell::RefCell<Option<String>>>,
            writes: Rc<std::cell::RefCell<Vec<String>>>,
        }

        impl velox_platform::PlatformClipboard for TestClipboard {
            fn read_text(&self) -> Option<String> {
                self.read_value.borrow().clone()
            }

            fn write_text(&self, text: &str) {
                self.writes.borrow_mut().push(text.to_owned());
            }
        }

        let read_value = Rc::new(std::cell::RefCell::new(Some(String::from("paste me"))));
        let writes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let clipboard = TestClipboard {
            read_value: read_value.clone(),
            writes: writes.clone(),
        };

        let mut handler = VeloxHandler::new(
            Runtime::new(),
            String::from("Test App"),
            vec![],
            None,
            None,
            None,
            false,
            Some(Box::new(clipboard)),
        );

        assert_eq!(handler.clipboard_read_text().as_deref(), Some("paste me"));

        handler.clipboard_write_text(String::from("copy me"));

        assert_eq!(writes.borrow().as_slice(), &[String::from("copy me")]);
    }
}
