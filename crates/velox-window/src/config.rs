#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title: String,
    pub width: f64,
    pub height: f64,
    pub min_width: Option<f64>,
    pub min_height: Option<f64>,
    pub max_width: Option<f64>,
    pub max_height: Option<f64>,
    pub resizable: bool,
    pub decorations: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("Velox"),
            width: 800.0,
            height: 600.0,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            resizable: true,
            decorations: true,
        }
    }
}

impl WindowConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_size(mut self, width: f64, height: f64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_min_size(mut self, width: f64, height: f64) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }

    pub fn with_max_size(mut self, width: f64, height: f64) -> Self {
        self.max_width = Some(width);
        self.max_height = Some(height);
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub fn to_window_attributes(&self) -> winit::window::WindowAttributes {
        let mut attrs = winit::window::WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
            .with_resizable(self.resizable)
            .with_decorations(self.decorations);

        if let (Some(min_w), Some(min_h)) = (self.min_width, self.min_height) {
            attrs = attrs.with_min_inner_size(winit::dpi::LogicalSize::new(min_w, min_h));
        }

        if let (Some(max_w), Some(max_h)) = (self.max_width, self.max_height) {
            attrs = attrs.with_max_inner_size(winit::dpi::LogicalSize::new(max_w, max_h));
        }

        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = WindowConfig::new();
        assert_eq!(config.title, "Velox");
        assert_eq!(config.width, 800.0);
        assert_eq!(config.height, 600.0);
        assert!(config.resizable);
        assert!(config.decorations);
        assert!(config.min_width.is_none());
        assert!(config.max_width.is_none());
    }

    #[test]
    fn builder_chain() {
        let config = WindowConfig::new()
            .with_title("My App")
            .with_size(1024.0, 768.0)
            .with_min_size(320.0, 240.0)
            .with_max_size(1920.0, 1080.0)
            .with_resizable(false)
            .with_decorations(false);

        assert_eq!(config.title, "My App");
        assert_eq!(config.width, 1024.0);
        assert_eq!(config.height, 768.0);
        assert_eq!(config.min_width, Some(320.0));
        assert_eq!(config.min_height, Some(240.0));
        assert_eq!(config.max_width, Some(1920.0));
        assert_eq!(config.max_height, Some(1080.0));
        assert!(!config.resizable);
        assert!(!config.decorations);
    }

    #[test]
    fn to_window_attributes_basic() {
        let config = WindowConfig::new().with_title("Test");
        let attrs = config.to_window_attributes();
        assert_eq!(attrs.title, "Test");
        assert!(attrs.resizable);
        assert!(attrs.decorations);
    }

    #[test]
    fn to_window_attributes_with_constraints() {
        let config = WindowConfig::new()
            .with_min_size(100.0, 100.0)
            .with_max_size(2000.0, 2000.0);
        let attrs = config.to_window_attributes();
        assert!(attrs.min_inner_size.is_some());
        assert!(attrs.max_inner_size.is_some());
    }
}
