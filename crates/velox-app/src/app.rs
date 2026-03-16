use velox_platform::{NativeClipboard, PlatformClipboard};
use velox_runtime::power::PowerPolicy;
use velox_runtime::Runtime;
use velox_scene::Scene;
use velox_style::ThemeManager;
use velox_window::WindowConfig;
use winit::event_loop::EventLoop;

use crate::handler::{UiRenderFn, VeloxHandler};

type SetupFn = Box<dyn FnMut(&mut Scene)>;

pub struct App {
    name: String,
    power_policy: PowerPolicy,
    window_configs: Vec<WindowConfig>,
    setup: Option<SetupFn>,
    ui_render: Option<UiRenderFn>,
    clipboard: Option<Box<dyn PlatformClipboard>>,
    theme_manager: Option<ThemeManager>,
    continuous_redraw: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            name: String::from("Velox App"),
            power_policy: PowerPolicy::default(),
            window_configs: Vec::new(),
            setup: None,
            ui_render: None,
            clipboard: None,
            theme_manager: None,
            continuous_redraw: false,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn power_policy(mut self, policy: PowerPolicy) -> Self {
        self.power_policy = policy;
        self
    }

    pub fn window(mut self, config: WindowConfig) -> Self {
        self.window_configs.push(config);
        self
    }

    pub fn window_configs(&self) -> &[WindowConfig] {
        &self.window_configs
    }

    /// Register a setup closure that initializes each window's scene.
    /// Called once per window, so the closure may be invoked multiple times
    /// in multi-window configurations. Use `FnMut` to allow state mutation
    /// across calls; avoid closures that consume captured values.
    pub fn setup(mut self, f: impl FnMut(&mut Scene) + 'static) -> Self {
        self.setup = Some(Box::new(f));
        self
    }

    pub fn setup_ui(
        mut self,
        f: impl FnMut() -> Vec<velox_ui::element::AnyElement> + 'static,
    ) -> Self {
        self.ui_render = Some(Box::new(f));
        self
    }

    pub fn clipboard(mut self, clipboard: impl PlatformClipboard + 'static) -> Self {
        self.clipboard = Some(Box::new(clipboard));
        self
    }

    pub fn theme_manager(mut self, manager: ThemeManager) -> Self {
        self.theme_manager = Some(manager);
        self
    }

    pub fn continuous_redraw(mut self) -> Self {
        self.continuous_redraw = true;
        self
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let Self {
            name,
            power_policy,
            window_configs,
            setup,
            ui_render,
            clipboard,
            theme_manager,
            continuous_redraw,
        } = self;
        let event_loop = EventLoop::new()?;
        let runtime = Runtime::builder().power_policy(power_policy).build();
        let clipboard = clipboard.unwrap_or_else(|| Box::new(NativeClipboard::new()));
        let mut handler = VeloxHandler::new(
            runtime,
            name,
            window_configs,
            setup,
            ui_render,
            theme_manager,
            continuous_redraw,
            Some(clipboard),
        );
        event_loop.run_app(&mut handler)?;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
