pub mod accessibility_bridge;
pub mod app;
pub mod clipboard;
pub mod file_dialog;
pub mod menu;
pub mod notification;
pub mod power;
mod stub;
pub mod tray;

pub use accessibility_bridge::{
    AccessibilityActionKind, AccessibilityActionRequest, AccessibilityBridge,
    WindowAccessibilityAdapter,
};
pub use app::PlatformApp;
pub use clipboard::{NativeClipboard, PlatformClipboard};
pub use file_dialog::{NativeFileDialog, PlatformFileDialog};
pub use menu::{MenuItem, PlatformMenu};
pub use notification::{NativeNotification, PlatformNotification};
pub use power::{BatteryState, PlatformPower, PowerSource};
pub use stub::StubPlatform;
pub use tray::PlatformTray;
