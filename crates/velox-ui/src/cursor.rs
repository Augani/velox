use crate::dispatch::DispatchTree;
use crate::style::CursorStyle;
use velox_scene::NodeId;

pub struct CursorManager {
    current: CursorStyle,
}

impl Default for CursorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorManager {
    pub fn new() -> Self {
        Self {
            current: CursorStyle::Default,
        }
    }

    pub fn current(&self) -> CursorStyle {
        self.current
    }

    pub fn resolve(&mut self, target: NodeId, dispatch: &DispatchTree) -> Option<CursorStyle> {
        let mut node = Some(target);
        while let Some(id) = node {
            if let Some(data) = dispatch.get(id)
                && let Some(cursor) = data.cursor {
                    if cursor != self.current {
                        self.current = cursor;
                        return Some(cursor);
                    }
                    return None;
                }
            node = dispatch.parent(id);
        }
        if self.current != CursorStyle::Default {
            self.current = CursorStyle::Default;
            return Some(CursorStyle::Default);
        }
        None
    }

    pub fn reset(&mut self) -> Option<CursorStyle> {
        if self.current != CursorStyle::Default {
            self.current = CursorStyle::Default;
            return Some(CursorStyle::Default);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::DispatchNodeData;

    #[test]
    fn initial_cursor_is_default() {
        let mgr = CursorManager::new();
        assert_eq!(mgr.current(), CursorStyle::Default);
    }

    #[test]
    fn resolve_returns_cursor_from_target() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let node = NodeId::from_raw_parts(1, 1);
        dispatch.register(
            node,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Pointer),
                ..Default::default()
            },
        );
        let result = mgr.resolve(node, &dispatch);
        assert_eq!(result, Some(CursorStyle::Pointer));
        assert_eq!(mgr.current(), CursorStyle::Pointer);
    }

    #[test]
    fn resolve_walks_ancestors() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let parent = NodeId::from_raw_parts(1, 1);
        let child = NodeId::from_raw_parts(2, 1);
        dispatch.register(
            parent,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Text),
                ..Default::default()
            },
        );
        dispatch.register(child, Some(parent), DispatchNodeData::default());
        let result = mgr.resolve(child, &dispatch);
        assert_eq!(result, Some(CursorStyle::Text));
    }

    #[test]
    fn resolve_returns_none_when_unchanged() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let node = NodeId::from_raw_parts(1, 1);
        dispatch.register(
            node,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Pointer),
                ..Default::default()
            },
        );
        mgr.resolve(node, &dispatch);
        let result = mgr.resolve(node, &dispatch);
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_resets_to_default_when_no_cursor() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let with_cursor = NodeId::from_raw_parts(1, 1);
        let without_cursor = NodeId::from_raw_parts(2, 1);
        dispatch.register(
            with_cursor,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Grab),
                ..Default::default()
            },
        );
        dispatch.register(without_cursor, None, DispatchNodeData::default());
        mgr.resolve(with_cursor, &dispatch);
        assert_eq!(mgr.current(), CursorStyle::Grab);
        let result = mgr.resolve(without_cursor, &dispatch);
        assert_eq!(result, Some(CursorStyle::Default));
        assert_eq!(mgr.current(), CursorStyle::Default);
    }

    #[test]
    fn resolve_no_change_when_already_default() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let node = NodeId::from_raw_parts(1, 1);
        dispatch.register(node, None, DispatchNodeData::default());
        let result = mgr.resolve(node, &dispatch);
        assert_eq!(result, None);
    }

    #[test]
    fn reset_returns_to_default() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let node = NodeId::from_raw_parts(1, 1);
        dispatch.register(
            node,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Wait),
                ..Default::default()
            },
        );
        mgr.resolve(node, &dispatch);
        let result = mgr.reset();
        assert_eq!(result, Some(CursorStyle::Default));
        assert_eq!(mgr.current(), CursorStyle::Default);
    }

    #[test]
    fn reset_noop_when_already_default() {
        let mgr_mut = &mut CursorManager::new();
        let result = mgr_mut.reset();
        assert_eq!(result, None);
    }

    #[test]
    fn child_cursor_overrides_parent() {
        let mut mgr = CursorManager::new();
        let mut dispatch = DispatchTree::new();
        let parent = NodeId::from_raw_parts(1, 1);
        let child = NodeId::from_raw_parts(2, 1);
        dispatch.register(
            parent,
            None,
            DispatchNodeData {
                cursor: Some(CursorStyle::Text),
                ..Default::default()
            },
        );
        dispatch.register(
            child,
            Some(parent),
            DispatchNodeData {
                cursor: Some(CursorStyle::Pointer),
                ..Default::default()
            },
        );
        let result = mgr.resolve(child, &dispatch);
        assert_eq!(result, Some(CursorStyle::Pointer));
    }

    #[test]
    fn all_cursor_variants_exist() {
        let variants = [
            CursorStyle::Default,
            CursorStyle::Pointer,
            CursorStyle::Text,
            CursorStyle::Grab,
            CursorStyle::Grabbing,
            CursorStyle::NotAllowed,
            CursorStyle::Move,
            CursorStyle::Crosshair,
            CursorStyle::Wait,
            CursorStyle::Progress,
            CursorStyle::Help,
            CursorStyle::ZoomIn,
            CursorStyle::ZoomOut,
            CursorStyle::ResizeN,
            CursorStyle::ResizeS,
            CursorStyle::ResizeE,
            CursorStyle::ResizeW,
            CursorStyle::ResizeNE,
            CursorStyle::ResizeNW,
            CursorStyle::ResizeSE,
            CursorStyle::ResizeSW,
            CursorStyle::ResizeEW,
            CursorStyle::ResizeNS,
        ];
        assert_eq!(variants.len(), 23);
    }
}
