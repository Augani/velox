#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(winit::window::WindowId);

impl WindowId {
    pub fn from_winit(id: winit::window::WindowId) -> Self {
        Self(id)
    }
}
