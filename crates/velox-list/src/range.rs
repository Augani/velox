#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportRange {
    pub start_index: usize,
    pub end_index: usize,
}

impl ViewportRange {
    pub fn len(&self) -> usize {
        self.end_index.saturating_sub(self.start_index)
    }

    pub fn is_empty(&self) -> bool {
        self.end_index <= self.start_index
    }

    pub fn contains(&self, index: usize) -> bool {
        index >= self.start_index && index < self.end_index
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedRanges {
    pub visible: ViewportRange,
    pub working: ViewportRange,
    pub prefetch: ViewportRange,
}

pub fn compute_expanded(
    visible: ViewportRange,
    total_count: usize,
    working_screens: f32,
    prefetch_screens: f32,
) -> ExpandedRanges {
    if total_count == 0 || visible.is_empty() {
        let empty = ViewportRange {
            start_index: 0,
            end_index: 0,
        };
        return ExpandedRanges {
            visible,
            working: empty,
            prefetch: empty,
        };
    }

    let visible_len = visible.len() as f32;
    let working_extend = (working_screens * visible_len).ceil() as usize;
    let working = ViewportRange {
        start_index: visible.start_index.saturating_sub(working_extend),
        end_index: (visible.end_index + working_extend).min(total_count),
    };

    let prefetch_extend = (prefetch_screens * visible_len).ceil() as usize;
    let prefetch = ViewportRange {
        start_index: working.start_index.saturating_sub(prefetch_extend),
        end_index: (working.end_index + prefetch_extend).min(total_count),
    };

    ExpandedRanges {
        visible,
        working,
        prefetch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_range_len_and_contains() {
        let range = ViewportRange {
            start_index: 3,
            end_index: 7,
        };
        assert_eq!(range.len(), 4);
        assert!(!range.is_empty());
        assert!(!range.contains(2));
        assert!(range.contains(3));
        assert!(range.contains(6));
        assert!(!range.contains(7));
    }

    #[test]
    fn empty_viewport_range() {
        let range = ViewportRange {
            start_index: 5,
            end_index: 5,
        };
        assert_eq!(range.len(), 0);
        assert!(range.is_empty());
        assert!(!range.contains(5));
    }

    #[test]
    fn expanded_ranges_computation() {
        let visible = ViewportRange {
            start_index: 10,
            end_index: 20,
        };
        let result = compute_expanded(visible, 100, 1.0, 1.0);

        assert_eq!(result.visible, visible);
        assert_eq!(result.working.start_index, 0);
        assert_eq!(result.working.end_index, 30);
        assert_eq!(result.prefetch.start_index, 0);
        assert_eq!(result.prefetch.end_index, 40);
    }

    #[test]
    fn expanded_ranges_clamp_to_bounds() {
        let visible = ViewportRange {
            start_index: 0,
            end_index: 5,
        };
        let result = compute_expanded(visible, 10, 1.0, 1.0);

        assert_eq!(result.working.start_index, 0);
        assert_eq!(result.working.end_index, 10);
        assert_eq!(result.prefetch.start_index, 0);
        assert_eq!(result.prefetch.end_index, 10);
    }

    #[test]
    fn expanded_ranges_empty_visible() {
        let visible = ViewportRange {
            start_index: 0,
            end_index: 0,
        };
        let result = compute_expanded(visible, 100, 1.0, 1.0);

        assert!(result.working.is_empty());
        assert!(result.prefetch.is_empty());
    }
}
