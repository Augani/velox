#[derive(Debug, Clone)]
pub struct WindowConfig {
    label: String,
    title: String,
    width: u32,
    height: u32,
    min_size: Option<(u32, u32)>,
    max_size: Option<(u32, u32)>,
    resizable: bool,
    decorations: bool,
}

impl WindowConfig {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            title: String::from("Velox"),
            width: 800,
            height: 600,
            min_size: None,
            max_size: None,
            resizable: true,
            decorations: true,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    pub fn max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some((width, height));
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    pub fn id_label(&self) -> &str {
        &self.label
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn get_min_size(&self) -> Option<(u32, u32)> {
        self.min_size
    }

    pub fn get_max_size(&self) -> Option<(u32, u32)> {
        self.max_size
    }

    pub fn is_resizable(&self) -> bool {
        self.resizable
    }

    pub fn has_decorations(&self) -> bool {
        self.decorations
    }

    pub fn to_window_attributes(&self) -> winit::window::WindowAttributes {
        let mut attrs = winit::window::WindowAttributes::default()
            .with_title(self.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(self.width),
                f64::from(self.height),
            ))
            .with_resizable(self.resizable)
            .with_decorations(self.decorations);

        if let Some((min_w, min_h)) = self.min_size {
            attrs = attrs.with_min_inner_size(winit::dpi::LogicalSize::new(
                f64::from(min_w),
                f64::from(min_h),
            ));
        }

        if let Some((max_w, max_h)) = self.max_size {
            attrs = attrs.with_max_inner_size(winit::dpi::LogicalSize::new(
                f64::from(max_w),
                f64::from(max_h),
            ));
        }

        attrs
    }
}
