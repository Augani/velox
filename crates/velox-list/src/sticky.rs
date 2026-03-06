use std::cell::Cell;

use crate::range::ViewportRange;

pub struct StickyHeaderState {
    header_indices: Vec<usize>,
    pinned_index: Cell<Option<usize>>,
}

impl StickyHeaderState {
    pub fn new(mut header_indices: Vec<usize>) -> Self {
        header_indices.sort_unstable();
        header_indices.dedup();
        Self {
            header_indices,
            pinned_index: Cell::new(None),
        }
    }

    pub fn update(&self, visible_range: ViewportRange) {
        if self.header_indices.is_empty() || visible_range.is_empty() {
            self.pinned_index.set(None);
            return;
        }

        let pinned = self
            .header_indices
            .iter()
            .rev()
            .find(|&&idx| idx <= visible_range.start_index)
            .copied();
        self.pinned_index.set(pinned);
    }

    pub fn pinned_index(&self) -> Option<usize> {
        self.pinned_index.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_header_tracks_scroll() {
        let state = StickyHeaderState::new(vec![0, 10, 20, 30]);

        state.update(ViewportRange {
            start_index: 0,
            end_index: 5,
        });
        assert_eq!(state.pinned_index(), Some(0));

        state.update(ViewportRange {
            start_index: 5,
            end_index: 15,
        });
        assert_eq!(state.pinned_index(), Some(0));

        state.update(ViewportRange {
            start_index: 12,
            end_index: 22,
        });
        assert_eq!(state.pinned_index(), Some(10));

        state.update(ViewportRange {
            start_index: 25,
            end_index: 35,
        });
        assert_eq!(state.pinned_index(), Some(20));
    }

    #[test]
    fn no_headers_returns_none() {
        let state = StickyHeaderState::new(vec![]);
        state.update(ViewportRange {
            start_index: 0,
            end_index: 10,
        });
        assert_eq!(state.pinned_index(), None);
    }

    #[test]
    fn empty_visible_range_returns_none() {
        let state = StickyHeaderState::new(vec![0, 10]);
        state.update(ViewportRange {
            start_index: 5,
            end_index: 5,
        });
        assert_eq!(state.pinned_index(), None);
    }
}
