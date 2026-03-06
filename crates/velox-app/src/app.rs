use velox_runtime::power::PowerPolicy;
use velox_runtime::Runtime;
use velox_window::WindowConfig;
use winit::event_loop::EventLoop;

use crate::handler::VeloxHandler;

pub struct App {
    name: String,
    power_policy: PowerPolicy,
    window_configs: Vec<WindowConfig>,
}

impl App {
    pub fn new() -> Self {
        Self {
            name: String::from("Velox App"),
            power_policy: PowerPolicy::default(),
            window_configs: Vec::new(),
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

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let runtime = Runtime::builder().power_policy(self.power_policy).build();
        let mut handler = VeloxHandler::new(runtime, self.window_configs);
        event_loop.run_app(&mut handler)?;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
