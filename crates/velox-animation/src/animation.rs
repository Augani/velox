use slotmap::new_key_type;

new_key_type! { pub struct AnimationId; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Running,
    Paused,
    Finished,
    Cancelled,
}
