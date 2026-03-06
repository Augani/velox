#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(winit::window::WindowId);

impl WindowId {
    pub(crate) fn from_winit(id: winit::window::WindowId) -> Self {
        Self(id)
    }

    pub(crate) fn winit_id(&self) -> winit::window::WindowId {
        self.0
    }
}
