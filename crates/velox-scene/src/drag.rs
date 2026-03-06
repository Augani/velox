use std::path::PathBuf;

use crate::geometry::Point;
use crate::node::NodeId;

#[derive(Debug, Clone)]
pub enum DragPayload {
    Files(Vec<PathBuf>),
    Text(String),
    Custom(String, Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    Began,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct DragEvent {
    pub payload: DragPayload,
    pub position: Point,
    pub phase: DragPhase,
}

pub struct DragState {
    pub source: Option<NodeId>,
    pub payload: Option<DragPayload>,
    pub position: Point,
    pub active: bool,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            source: None,
            payload: None,
            position: Point::new(0.0, 0.0),
            active: false,
        }
    }

    pub fn start(&mut self, source: NodeId, payload: DragPayload, position: Point) {
        self.source = Some(source);
        self.payload = Some(payload);
        self.position = position;
        self.active = true;
    }

    pub fn update_position(&mut self, position: Point) {
        self.position = position;
    }

    pub fn finish(&mut self) -> Option<DragPayload> {
        self.active = false;
        self.source = None;
        self.payload.take()
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.source = None;
        self.payload = None;
    }
}

impl Default for DragState {
    fn default() -> Self {
        Self::new()
    }
}

pub trait DropTarget: 'static {
    fn accepts(&self, payload: &DragPayload) -> bool;
    fn on_drop(&mut self, payload: DragPayload, position: Point) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn drag_state_lifecycle() {
        let mut sm: SlotMap<NodeId, ()> = SlotMap::with_key();
        let source = sm.insert(());
        let mut state = DragState::new();
        assert!(!state.active);

        state.start(
            source,
            DragPayload::Text("hello".into()),
            Point::new(10.0, 20.0),
        );
        assert!(state.active);
        assert_eq!(state.source, Some(source));

        state.update_position(Point::new(50.0, 60.0));
        assert_eq!(state.position, Point::new(50.0, 60.0));

        let payload = state.finish();
        assert!(payload.is_some());
        assert!(!state.active);
    }

    #[test]
    fn drag_cancel_clears_state() {
        let mut sm: SlotMap<NodeId, ()> = SlotMap::with_key();
        let source = sm.insert(());
        let mut state = DragState::new();
        state.start(source, DragPayload::Files(vec![]), Point::new(0.0, 0.0));
        state.cancel();
        assert!(!state.active);
        assert!(state.payload.is_none());
    }

    #[test]
    fn custom_payload() {
        let payload = DragPayload::Custom("image/png".into(), vec![0x89, 0x50, 0x4E, 0x47]);
        if let DragPayload::Custom(mime, data) = &payload {
            assert_eq!(mime, "image/png");
            assert_eq!(data.len(), 4);
        } else {
            panic!("expected Custom");
        }
    }
}
