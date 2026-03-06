pub trait HeightProvider {
    fn height_for_index(&self, index: usize) -> f32;
    fn count(&self) -> usize;
}

pub struct FixedHeight {
    pub height: f32,
    pub count: usize,
}

impl HeightProvider for FixedHeight {
    fn height_for_index(&self, _index: usize) -> f32 {
        self.height
    }

    fn count(&self) -> usize {
        self.count
    }
}

pub struct CumulativeHeightCache {
    prefix_sums: Vec<f32>,
}

impl CumulativeHeightCache {
    pub fn from_provider(provider: &dyn HeightProvider) -> Self {
        let count = provider.count();
        let mut prefix_sums = Vec::with_capacity(count + 1);
        prefix_sums.push(0.0);

        let mut cumulative = 0.0;
        for i in 0..count {
            cumulative += provider.height_for_index(i);
            prefix_sums.push(cumulative);
        }

        Self { prefix_sums }
    }

    pub fn total_height(&self) -> f32 {
        self.prefix_sums.last().copied().unwrap_or(0.0)
    }

    pub fn offset_for_index(&self, index: usize) -> f32 {
        if index >= self.prefix_sums.len() {
            return self.total_height();
        }
        self.prefix_sums[index]
    }

    pub fn index_at_offset(&self, offset: f32) -> usize {
        if self.prefix_sums.len() <= 1 {
            return 0;
        }

        let result = self.prefix_sums.partition_point(|&sum| sum <= offset);
        result.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_height_provider() {
        let provider = FixedHeight {
            height: 40.0,
            count: 10,
        };
        assert_eq!(provider.count(), 10);
        assert_eq!(provider.height_for_index(0), 40.0);
        assert_eq!(provider.height_for_index(9), 40.0);
    }

    #[test]
    fn cumulative_cache_from_fixed_height() {
        let provider = FixedHeight {
            height: 50.0,
            count: 4,
        };
        let cache = CumulativeHeightCache::from_provider(&provider);

        assert_eq!(cache.total_height(), 200.0);
        assert_eq!(cache.offset_for_index(0), 0.0);
        assert_eq!(cache.offset_for_index(1), 50.0);
        assert_eq!(cache.offset_for_index(2), 100.0);
        assert_eq!(cache.offset_for_index(3), 150.0);
    }

    struct VariableHeight(Vec<f32>);

    impl HeightProvider for VariableHeight {
        fn height_for_index(&self, index: usize) -> f32 {
            self.0.get(index).copied().unwrap_or(0.0)
        }

        fn count(&self) -> usize {
            self.0.len()
        }
    }

    #[test]
    fn cumulative_cache_variable_heights() {
        let provider = VariableHeight(vec![10.0, 20.0, 30.0, 40.0]);
        let cache = CumulativeHeightCache::from_provider(&provider);

        assert_eq!(cache.total_height(), 100.0);
        assert_eq!(cache.offset_for_index(0), 0.0);
        assert_eq!(cache.offset_for_index(1), 10.0);
        assert_eq!(cache.offset_for_index(2), 30.0);
        assert_eq!(cache.offset_for_index(3), 60.0);
    }

    #[test]
    fn index_at_offset_binary_search() {
        let provider = FixedHeight {
            height: 50.0,
            count: 10,
        };
        let cache = CumulativeHeightCache::from_provider(&provider);

        assert_eq!(cache.index_at_offset(0.0), 0);
        assert_eq!(cache.index_at_offset(25.0), 0);
        assert_eq!(cache.index_at_offset(50.0), 1);
        assert_eq!(cache.index_at_offset(75.0), 1);
        assert_eq!(cache.index_at_offset(100.0), 2);
        assert_eq!(cache.index_at_offset(499.0), 9);
    }

    #[test]
    fn offset_for_index_out_of_bounds() {
        let provider = FixedHeight {
            height: 50.0,
            count: 3,
        };
        let cache = CumulativeHeightCache::from_provider(&provider);

        assert_eq!(cache.offset_for_index(100), 150.0);
    }

    #[test]
    fn empty_provider() {
        let provider = FixedHeight {
            height: 50.0,
            count: 0,
        };
        let cache = CumulativeHeightCache::from_provider(&provider);

        assert_eq!(cache.total_height(), 0.0);
        assert_eq!(cache.index_at_offset(0.0), 0);
    }
}
