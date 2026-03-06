pub mod animation;
pub mod crossfade;
pub mod easing;
pub mod interpolate;
pub mod keyframes;
pub mod manager;
pub mod spring;
pub mod tween;

pub use animation::{AnimationId, AnimationState};
pub use crossfade::Crossfade;
pub use easing::Easing;
pub use interpolate::Interpolatable;
pub use keyframes::{KeyframeEntry, Keyframes};
pub use manager::AnimationManager;
pub use spring::{Spring, SpringConfig, SpringValue};
pub use tween::Tween;
