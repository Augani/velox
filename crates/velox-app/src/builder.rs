use velox_runtime::power::PowerPolicy;
use velox_window::WindowConfig;

pub struct App {
    name: String,
    power_policy: PowerPolicy,
    window_configs: Vec<WindowConfig>,
}

impl App {
    pub fn builder(name: impl Into<String>) -> AppBuilder {
        AppBuilder {
            name: name.into(),
            power_policy: PowerPolicy::default(),
            window_configs: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn power_policy(&self) -> PowerPolicy {
        self.power_policy
    }

    pub fn window_configs(&self) -> &[WindowConfig] {
        &self.window_configs
    }
}

pub struct AppBuilder {
    name: String,
    power_policy: PowerPolicy,
    window_configs: Vec<WindowConfig>,
}

impl AppBuilder {
    pub fn power_policy(mut self, policy: PowerPolicy) -> Self {
        self.power_policy = policy;
        self
    }

    pub fn window(mut self, config: WindowConfig) -> Self {
        self.window_configs.push(config);
        self
    }

    pub fn build(self) -> App {
        App {
            name: self.name,
            power_policy: self.power_policy,
            window_configs: self.window_configs,
        }
    }
}
