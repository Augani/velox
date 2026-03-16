use std::any::{Any, TypeId};
use std::sync::Arc;
use velox_scene::{NodeId, Point};

const DRAG_THRESHOLD: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhaseUi {
    Idle,
    Pending,
    Active,
}

pub struct DragManager {
    phase: DragPhaseUi,
    mousedown_pos: Point,
    source_node: Option<NodeId>,
    drag_value: Option<Arc<dyn Any + Send + Sync>>,
    drag_type_id: Option<TypeId>,
    current_pos: Point,
}

impl Default for DragManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DragManager {
    pub fn new() -> Self {
        Self {
            phase: DragPhaseUi::Idle,
            mousedown_pos: Point::new(0.0, 0.0),
            source_node: None,
            drag_value: None,
            drag_type_id: None,
            current_pos: Point::new(0.0, 0.0),
        }
    }

    pub fn phase(&self) -> DragPhaseUi {
        self.phase
    }

    pub fn source_node(&self) -> Option<NodeId> {
        self.source_node
    }

    pub fn drag_value(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.drag_value.as_ref()
    }

    pub fn drag_type_id(&self) -> Option<TypeId> {
        self.drag_type_id
    }

    pub fn current_position(&self) -> Point {
        self.current_pos
    }

    pub fn begin_pending(&mut self, node: NodeId, position: Point) {
        self.phase = DragPhaseUi::Pending;
        self.mousedown_pos = position;
        self.source_node = Some(node);
        self.current_pos = position;
    }

    pub fn mouse_move(
        &mut self,
        position: Point,
        value_factory: impl FnOnce() -> Option<(Arc<dyn Any + Send + Sync>, TypeId)>,
    ) -> bool {
        self.current_pos = position;
        match self.phase {
            DragPhaseUi::Pending => {
                let dx = position.x - self.mousedown_pos.x;
                let dy = position.y - self.mousedown_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance >= DRAG_THRESHOLD {
                    if let Some((value, type_id)) = value_factory() {
                        self.phase = DragPhaseUi::Active;
                        self.drag_value = Some(value);
                        self.drag_type_id = Some(type_id);
                        return true;
                    }
                    self.cancel();
                }
                false
            }
            DragPhaseUi::Active => true,
            DragPhaseUi::Idle => false,
        }
    }

    pub fn finish(&mut self) -> Option<DragFinish> {
        if self.phase != DragPhaseUi::Active {
            self.cancel();
            return None;
        }
        let result = DragFinish {
            source_node: self.source_node,
            value: self.drag_value.take(),
            type_id: self.drag_type_id.take(),
            position: self.current_pos,
        };
        self.reset();
        Some(result)
    }

    pub fn cancel(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.phase = DragPhaseUi::Idle;
        self.source_node = None;
        self.drag_value = None;
        self.drag_type_id = None;
    }
}

pub struct DragFinish {
    pub source_node: Option<NodeId>,
    pub value: Option<Arc<dyn Any + Send + Sync>>,
    pub type_id: Option<TypeId>,
    pub position: Point,
}

#[allow(clippy::type_complexity)]
pub struct DropTargetEntry {
    pub node: NodeId,
    pub type_id: TypeId,
    pub can_drop: Option<Box<dyn Fn(&dyn Any) -> bool>>,
}

pub fn resolve_drop_target<'a>(
    drag_type_id: TypeId,
    drag_value: &dyn Any,
    targets: &'a [DropTargetEntry],
    target_node: NodeId,
    ancestor_fn: impl Fn(NodeId) -> Option<NodeId>,
) -> Option<&'a DropTargetEntry> {
    let mut current = Some(target_node);
    while let Some(node) = current {
        for entry in targets {
            if entry.node != node {
                continue;
            }
            if entry.type_id != drag_type_id {
                continue;
            }
            match &entry.can_drop {
                Some(predicate) if !predicate(drag_value) => continue,
                _ => return Some(entry),
            }
        }
        current = ancestor_fn(node);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn make_ids(count: usize) -> Vec<NodeId> {
        let mut sm: SlotMap<NodeId, ()> = SlotMap::with_key();
        (0..count).map(|_| sm.insert(())).collect()
    }

    #[test]
    fn initial_state_is_idle() {
        let mgr = DragManager::new();
        assert_eq!(mgr.phase(), DragPhaseUi::Idle);
        assert!(mgr.source_node().is_none());
        assert!(mgr.drag_value().is_none());
    }

    #[test]
    fn pending_below_threshold_stays_pending() {
        let mut mgr = DragManager::new();
        let ids = make_ids(1);
        mgr.begin_pending(ids[0], Point::new(10.0, 10.0));
        assert_eq!(mgr.phase(), DragPhaseUi::Pending);

        let activated = mgr.mouse_move(Point::new(11.0, 11.0), || {
            Some((
                Arc::new(42u32) as Arc<dyn Any + Send + Sync>,
                TypeId::of::<u32>(),
            ))
        });
        assert!(!activated);
        assert_eq!(mgr.phase(), DragPhaseUi::Pending);
    }

    #[test]
    fn exceeds_threshold_activates_drag() {
        let mut mgr = DragManager::new();
        let ids = make_ids(1);
        mgr.begin_pending(ids[0], Point::new(10.0, 10.0));

        let activated = mgr.mouse_move(Point::new(20.0, 10.0), || {
            Some((
                Arc::new(42u32) as Arc<dyn Any + Send + Sync>,
                TypeId::of::<u32>(),
            ))
        });
        assert!(activated);
        assert_eq!(mgr.phase(), DragPhaseUi::Active);
        assert_eq!(mgr.source_node(), Some(ids[0]));
    }

    #[test]
    fn drag_finish_returns_value() {
        let mut mgr = DragManager::new();
        let ids = make_ids(1);
        mgr.begin_pending(ids[0], Point::new(0.0, 0.0));
        mgr.mouse_move(Point::new(10.0, 0.0), || {
            Some((
                Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>,
                TypeId::of::<String>(),
            ))
        });

        let result = mgr.finish();
        assert!(result.is_some());
        let finish = result.unwrap();
        assert_eq!(finish.source_node, Some(ids[0]));
        assert!(finish.value.is_some());
        let val = finish.value.unwrap();
        assert_eq!(*val.downcast_ref::<String>().unwrap(), "hello");
        assert_eq!(mgr.phase(), DragPhaseUi::Idle);
    }

    #[test]
    fn drag_cancel_resets_to_idle() {
        let mut mgr = DragManager::new();
        let ids = make_ids(1);
        mgr.begin_pending(ids[0], Point::new(0.0, 0.0));
        mgr.mouse_move(Point::new(10.0, 0.0), || {
            Some((
                Arc::new(1u32) as Arc<dyn Any + Send + Sync>,
                TypeId::of::<u32>(),
            ))
        });
        assert_eq!(mgr.phase(), DragPhaseUi::Active);

        mgr.cancel();
        assert_eq!(mgr.phase(), DragPhaseUi::Idle);
        assert!(mgr.source_node().is_none());
        assert!(mgr.drag_value().is_none());
    }

    #[test]
    fn finish_on_pending_returns_none() {
        let mut mgr = DragManager::new();
        let ids = make_ids(1);
        mgr.begin_pending(ids[0], Point::new(0.0, 0.0));
        let result = mgr.finish();
        assert!(result.is_none());
        assert_eq!(mgr.phase(), DragPhaseUi::Idle);
    }

    #[test]
    fn no_value_factory_cancels_drag() {
        let mut mgr = DragManager::new();
        let ids = make_ids(1);
        mgr.begin_pending(ids[0], Point::new(0.0, 0.0));
        let activated = mgr.mouse_move(Point::new(10.0, 0.0), || None);
        assert!(!activated);
        assert_eq!(mgr.phase(), DragPhaseUi::Idle);
    }

    #[test]
    fn resolve_drop_target_type_match() {
        let ids = make_ids(3);
        let targets = vec![
            DropTargetEntry {
                node: ids[0],
                type_id: TypeId::of::<String>(),
                can_drop: None,
            },
            DropTargetEntry {
                node: ids[1],
                type_id: TypeId::of::<u32>(),
                can_drop: None,
            },
        ];

        let value: u32 = 42;
        let result = resolve_drop_target(TypeId::of::<u32>(), &value, &targets, ids[2], |node| {
            if node == ids[2] {
                Some(ids[1])
            } else if node == ids[1] {
                Some(ids[0])
            } else {
                None
            }
        });
        assert!(result.is_some());
        assert_eq!(result.unwrap().node, ids[1]);
    }

    #[test]
    fn resolve_drop_target_can_drop_predicate() {
        let ids = make_ids(2);
        let targets = vec![DropTargetEntry {
            node: ids[0],
            type_id: TypeId::of::<u32>(),
            can_drop: Some(Box::new(|val| {
                val.downcast_ref::<u32>().map_or(false, |v| *v > 10)
            })),
        }];

        let rejected_value: u32 = 5;
        let result = resolve_drop_target(
            TypeId::of::<u32>(),
            &rejected_value,
            &targets,
            ids[0],
            |_| None,
        );
        assert!(result.is_none());

        let accepted_value: u32 = 20;
        let result = resolve_drop_target(
            TypeId::of::<u32>(),
            &accepted_value,
            &targets,
            ids[0],
            |_| None,
        );
        assert!(result.is_some());
    }

    #[test]
    fn resolve_drop_target_no_match() {
        let ids = make_ids(1);
        let targets = vec![DropTargetEntry {
            node: ids[0],
            type_id: TypeId::of::<String>(),
            can_drop: None,
        }];

        let value: u32 = 42;
        let result = resolve_drop_target(TypeId::of::<u32>(), &value, &targets, ids[0], |_| None);
        assert!(result.is_none());
    }
}
