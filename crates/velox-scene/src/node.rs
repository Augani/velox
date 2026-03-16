use slotmap::new_key_type;

new_key_type! {
    pub struct NodeId;
}

impl NodeId {
    /// Construct a `NodeId` from raw index and version components.
    /// The caller must ensure the resulting key is valid in the target `SlotMap`,
    /// otherwise lookups will silently return stale data or `None`.
    /// Intended for testing and FFI/accessibility bridging only.
    pub fn from_raw_parts(index: u32, version: u32) -> Self {
        let ffi = (version as u64) << 32 | index as u64;
        Self(slotmap::KeyData::from_ffi(ffi))
    }
}
