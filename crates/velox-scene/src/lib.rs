mod geometry;
mod node;
mod tree;

pub use geometry::{Point, Rect, Size};
pub use node::NodeId;
pub use tree::{EventDispatchResult, NodeTree};

mod layout;
mod paint;
mod painter;

pub use layout::{Direction, Layout, PaddingLayout, StackLayout};
pub use paint::{
    BlendMode, Color, CommandList, GlyphUpload, Gradient, GradientStop, PaintCommand,
    PositionedGlyph, TextureId,
};
pub use painter::Painter;

mod accessibility;
pub use accessibility::{
    AccessibilityNode, AccessibilityRole, AccessibilityTreeNode, AccessibilityTreeSnapshot,
};

mod focus;
mod hit_test;
mod overlay;
mod scene;

pub use focus::{FocusChange, FocusState};
pub use overlay::{ModalConfig, OverlayId, OverlayStack};
pub use scene::Scene;

mod shortcut;

pub use shortcut::{Key, KeyCombo, Modifiers, ShortcutId, ShortcutRegistry};

mod drag;
mod event;
mod event_handler;
mod ime;

pub use drag::{DragEvent, DragPayload, DragPhase, DragState, DropTarget};
pub use event::{ButtonState, KeyEvent, KeyState, MouseButton, MouseEvent, ScrollEvent};
pub use event_handler::{EventContext, EventHandler};
pub use ime::ImeEvent;
