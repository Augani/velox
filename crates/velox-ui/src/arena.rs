pub struct RenderArena {
    bump: bumpalo::Bump,
}

impl RenderArena {
    pub fn new() -> Self {
        Self {
            bump: bumpalo::Bump::new(),
        }
    }

    pub fn reset(&mut self) {
        self.bump.reset();
    }

    pub fn alloc_str(&self, s: &str) -> &str {
        self.bump.alloc_str(s)
    }

    pub fn alloc_slice<T: Copy>(&self, slice: &[T]) -> &[T] {
        self.bump.alloc_slice_copy(slice)
    }

    pub fn bytes_allocated(&self) -> usize {
        self.bump.allocated_bytes()
    }
}

impl Default for RenderArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_allocates_and_resets() {
        let mut arena = RenderArena::new();
        let s = arena.alloc_str("hello");
        assert_eq!(s, "hello");
        assert!(arena.bytes_allocated() > 0);

        arena.reset();
    }

    #[test]
    fn arena_slice_alloc() {
        let arena = RenderArena::new();
        let data = [1.0f32, 2.0, 3.0];
        let allocated = arena.alloc_slice(&data);
        assert_eq!(allocated, &[1.0, 2.0, 3.0]);
    }
}
