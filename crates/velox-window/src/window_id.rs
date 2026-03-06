use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(winit::window::WindowId);

impl WindowId {
    pub fn from_winit(id: winit::window::WindowId) -> Self {
        Self(id)
    }

    pub fn as_winit(&self) -> winit::window::WindowId {
        self.0
    }
}

impl From<winit::window::WindowId> for WindowId {
    fn from(id: winit::window::WindowId) -> Self {
        Self(id)
    }
}

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowId({:?})", self.0)
    }
}
