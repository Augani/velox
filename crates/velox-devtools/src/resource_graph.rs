use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ResourceNode {
    GlyphAtlas {
        width: u32,
        height: u32,
        bytes_used: u64,
        glyph_count: usize,
    },
    TexturePool {
        texture_count: usize,
        bytes_used: u64,
        max_bytes: u64,
    },
    CacheStore {
        entry_count: usize,
        bytes_used: u64,
        max_bytes: u64,
        pressure: String,
    },
    AnimationPool {
        running_count: usize,
        total_registered: usize,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ResourceGraph {
    nodes: HashMap<String, ResourceNode>,
}

impl ResourceGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, name: impl Into<String>, node: ResourceNode) {
        self.nodes.insert(name.into(), node);
    }

    pub fn get(&self, name: &str) -> Option<&ResourceNode> {
        self.nodes.get(name)
    }

    pub fn snapshot(&self) -> HashMap<String, ResourceNode> {
        self.nodes.clone()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn total_bytes(&self) -> u64 {
        self.nodes
            .values()
            .map(|n| match n {
                ResourceNode::GlyphAtlas { bytes_used, .. } => *bytes_used,
                ResourceNode::TexturePool { bytes_used, .. } => *bytes_used,
                ResourceNode::CacheStore { bytes_used, .. } => *bytes_used,
                ResourceNode::AnimationPool { .. } => 0,
            })
            .sum()
    }

    pub fn diff(&self, other: &ResourceGraph) -> Vec<ResourceChange> {
        let mut changes = Vec::new();
        for (name, node) in &self.nodes {
            let old_bytes = other.nodes.get(name).map(node_bytes).unwrap_or(0);
            let new_bytes = node_bytes(node);
            if old_bytes != new_bytes {
                changes.push(ResourceChange {
                    name: name.clone(),
                    old_bytes,
                    new_bytes,
                });
            }
        }
        for name in other.nodes.keys() {
            if !self.nodes.contains_key(name) {
                changes.push(ResourceChange {
                    name: name.clone(),
                    old_bytes: node_bytes(other.nodes.get(name).unwrap()),
                    new_bytes: 0,
                });
            }
        }
        changes
    }
}

#[derive(Debug, Clone)]
pub struct ResourceChange {
    pub name: String,
    pub old_bytes: u64,
    pub new_bytes: u64,
}

fn node_bytes(node: &ResourceNode) -> u64 {
    match node {
        ResourceNode::GlyphAtlas { bytes_used, .. } => *bytes_used,
        ResourceNode::TexturePool { bytes_used, .. } => *bytes_used,
        ResourceNode::CacheStore { bytes_used, .. } => *bytes_used,
        ResourceNode::AnimationPool { .. } => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_retrieve() {
        let mut graph = ResourceGraph::new();
        graph.record(
            "glyph_atlas",
            ResourceNode::GlyphAtlas {
                width: 1024,
                height: 1024,
                bytes_used: 1024 * 1024,
                glyph_count: 256,
            },
        );
        assert_eq!(graph.node_count(), 1);
        assert!(graph.get("glyph_atlas").is_some());
    }

    #[test]
    fn total_bytes_sums_all() {
        let mut graph = ResourceGraph::new();
        graph.record(
            "atlas",
            ResourceNode::GlyphAtlas {
                width: 512,
                height: 512,
                bytes_used: 1000,
                glyph_count: 50,
            },
        );
        graph.record(
            "textures",
            ResourceNode::TexturePool {
                texture_count: 5,
                bytes_used: 2000,
                max_bytes: 10000,
            },
        );
        assert_eq!(graph.total_bytes(), 3000);
    }

    #[test]
    fn diff_detects_changes() {
        let mut old = ResourceGraph::new();
        old.record(
            "atlas",
            ResourceNode::GlyphAtlas {
                width: 512,
                height: 512,
                bytes_used: 1000,
                glyph_count: 50,
            },
        );

        let mut new_graph = ResourceGraph::new();
        new_graph.record(
            "atlas",
            ResourceNode::GlyphAtlas {
                width: 1024,
                height: 1024,
                bytes_used: 4000,
                glyph_count: 200,
            },
        );

        let changes = new_graph.diff(&old);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].old_bytes, 1000);
        assert_eq!(changes[0].new_bytes, 4000);
    }

    #[test]
    fn diff_detects_removal() {
        let mut old = ResourceGraph::new();
        old.record(
            "removed",
            ResourceNode::TexturePool {
                texture_count: 2,
                bytes_used: 500,
                max_bytes: 1000,
            },
        );

        let new_graph = ResourceGraph::new();
        let changes = new_graph.diff(&old);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].new_bytes, 0);
    }

    #[test]
    fn snapshot_returns_clone() {
        let mut graph = ResourceGraph::new();
        graph.record(
            "animations",
            ResourceNode::AnimationPool {
                running_count: 3,
                total_registered: 10,
            },
        );
        let snap = graph.snapshot();
        assert_eq!(snap.len(), 1);
    }
}
