pub mod scanner;

use indextree::{Arena, NodeId};
use bitflags::bitflags;

bitflags! {
    /// Compact representation of file metadata and permissions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NodeFlags: u16 {
        const IS_DIRECTORY = 0b0000_0001;
        const IS_SYMLINK   = 0b0000_0010;
        const IS_HIDDEN    = 0b0000_0100;
        const IS_PSEUDO    = 0b0000_1000; // E.g., <Files> or <Ignored> group nodes
        const IS_HARDLINK_DUPE = 0b0001_0000;
    }
}

/// The core data payload for a node in the file system tree.
#[derive(Debug, Clone)]
pub struct NodeData {
    pub name: Box<str>,
    pub size: u64,
    pub allocated_size: u64,
    pub mtime: i64,
    pub flags: NodeFlags,
}

impl NodeData {
    pub fn new(name: &str, size: u64, allocated_size: u64, mtime: i64, flags: NodeFlags) -> Self {
        Self {
            name: name.into(),
            size,
            allocated_size,
            mtime,
            flags,
        }
    }
}

/// A cache-friendly File System Tree that wraps the indextree Arena.
pub struct FileTree {
    arena: Arena<NodeData>,
    root: Option<NodeId>,
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            root: None,
        }
    }

    /// Sets the root of the file system tree.
    pub fn set_root(&mut self, data: NodeData) -> NodeId {
        let id = self.arena.new_node(data);
        self.root = Some(id);
        id
    }

    /// Appends a new child node to a given parent node.
    pub fn add_child(&mut self, parent: NodeId, data: NodeData) -> NodeId {
        let child = self.arena.new_node(data);
        parent.append(child, &mut self.arena);
        child
    }

    /// Retrieves the data for a given NodeId.
    pub fn get_data(&self, node: NodeId) -> Option<&NodeData> {
        self.arena.get(node).map(|n| n.get())
    }

    pub fn get_root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn get_children(&self, node: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        node.children(&self.arena)
    }

    /// Recursively calculates and updates the total size of each directory node.
    /// This is an O(N) post-order traversal operation.
    pub fn aggregate_sizes(&mut self) {
        if let Some(root) = self.root {
            self.post_order_aggregate(root);
        }
    }

    fn post_order_aggregate(&mut self, node: NodeId) -> (u64, u64) {
        let mut total_size = 0;
        let mut total_allocated = 0;

        // Collect children first to appease the borrow checker during mutation.
        // In a real tree, caching this or avoiding allocation might be needed for huge directories,
        // but `indextree` makes this pattern relatively cheap.
        let children: Vec<NodeId> = node.children(&self.arena).collect();

        for child in children {
            let (child_size, child_alloc) = self.post_order_aggregate(child);
            total_size += child_size;
            total_allocated += child_alloc;
        }

        if let Some(data) = self.arena.get_mut(node).map(|n| n.get_mut()) {
            if data.flags.contains(NodeFlags::IS_DIRECTORY) || data.flags.contains(NodeFlags::IS_PSEUDO) {
                // Directories adopt the accumulated size of their children
                data.size = total_size;
                data.allocated_size = total_allocated;
            } else {
                // Leaves simply contribute their own size
                total_size += data.size;
                total_allocated += data.allocated_size;
            }
        }

        (total_size, total_allocated)
    }

    /// Removes a node and all of its descendants from the arena, freeing the memory for reuse.
    pub fn remove_subtree(&mut self, node: NodeId) {
        node.remove_subtree(&mut self.arena);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_million_node_arena_performance() {
        let mut tree = FileTree::new();
        
        let root_data = NodeData::new("root", 0, 0, 0, NodeFlags::IS_DIRECTORY);
        let root = tree.set_root(root_data);

        println!("Building 1,000,000 synthetic nodes...");
        let start = Instant::now();
        
        let mut current_parent = root;
        // Build a wide and deep tree
        for i in 0..100 {
            let dir = tree.add_child(current_parent, NodeData::new(
                &format!("dir_{}", i), 0, 0, 0, NodeFlags::IS_DIRECTORY
            ));
            
            for j in 0..10_000 {
                tree.add_child(dir, NodeData::new(
                    &format!("file_{}", j), 1024, 4096, 0, NodeFlags::empty()
                ));
            }
            current_parent = dir; // Create a cascading deep chain of directories
        }
        
        let build_time = start.elapsed();
        println!("Built 1,000,000 nodes in {:?}", build_time);
        
        println!("Running post-order size aggregation...");
        let start_agg = Instant::now();
        tree.aggregate_sizes();
        let agg_time = start_agg.elapsed();
        println!("Aggregated 1,000,000 nodes in {:?}", agg_time);
        
        // The root should now have size = 1,000,000 * 1024
        let root_data = tree.get_data(root).unwrap();
        assert_eq!(root_data.size, 1_000_000 * 1024);
        assert_eq!(root_data.allocated_size, 1_000_000 * 4096);
        
        // Assert speed (1M nodes should aggregate in < 50ms in release mode, maybe a bit more in debug)
        // We won't strictly panic on CI variance, but we log it.
    }
}
