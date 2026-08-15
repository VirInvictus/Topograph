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
}
