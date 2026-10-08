//! topograph-core: the headless scanner and arena behind Topograph.
//! Pure Rust, no Qt: it walks directories and models the resulting tree;
//! the GUI crate consumes it over the CXX-Qt bridge.

pub mod scanner;
pub mod treemap;

pub use indextree::NodeId;

use bitflags::bitflags;
use indextree::Arena;

bitflags! {
    /// Compact representation of file metadata flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NodeFlags: u16 {
        const IS_DIRECTORY = 0b0000_0001;
        const IS_SYMLINK   = 0b0000_0010;
        const IS_HIDDEN    = 0b0000_0100; // Phase 17 scaffold: never set or read yet
        const IS_PSEUDO    = 0b0000_1000; // Phase 7 scaffold (<Files>/<Ignored>): the aggregation arm reads it, nothing sets it yet
        const IS_HARDLINK_DUPE = 0b0001_0000;
    }
}

/// The core data payload for a node in the file system tree.
#[derive(Debug, Clone)]
pub struct NodeData {
    pub name: Box<str>,
    pub size: u64,
    /// Apparent vs allocated: captured at scan time, aggregated in post-order,
    /// and summed into the scan summary (root-level; the per-node value has no
    /// UI surface yet).
    pub allocated_size: u64,
    /// Captured at scan time (the only cheap moment).
    pub mtime: i64,
    /// Items (files + directories) in this subtree; leaves carry 1.
    /// Written by `aggregate_sizes`, zero before it runs.
    pub count: usize,
    /// This node's share of its parent's aggregated size, 0-100. Written by
    /// the second aggregation pass; 0 until it runs. The root carries 100
    /// except on a zero-size tree (an empty scan), where a zero parent
    /// leaves it 0; the GUI shows the root row at 100 regardless.
    pub percent: f32,
    /// Cumulative share of the parent's size taken by this node's preceding
    /// siblings, 0-100: the node's offset within its sibling run for radial
    /// geometry (Phase 10's consumer). Written with `percent`.
    pub rel_start: f32,
    /// Oldest/newest entry mtime below this node, folded in post-order over
    /// the children (a directory's own mtime is not part of its fold, so it
    /// describes when the subtree's contents last changed). Leaves and
    /// empty directories carry their own mtime in both fields.
    pub mtime_oldest: i64,
    pub mtime_newest: i64,
    pub flags: NodeFlags,
}

impl NodeData {
    pub fn new(name: &str, size: u64, allocated_size: u64, mtime: i64, flags: NodeFlags) -> Self {
        Self {
            name: name.into(),
            size,
            allocated_size,
            mtime,
            count: 0,
            percent: 0.0,
            rel_start: 0.0,
            mtime_oldest: mtime,
            mtime_newest: mtime,
            flags,
        }
    }
}

/// A share in percent of a total: `size / parent * 100`. A zero parent
/// returns 0 rather than dividing (an empty directory's children all read 0).
pub fn percent_of(size: u64, parent: u64) -> f32 {
    if parent == 0 {
        0.0
    } else {
        (size as f64 / parent as f64 * 100.0) as f32
    }
}

/// A cache-friendly File System Tree that wraps the indextree Arena.
pub struct FileTree {
    arena: Arena<NodeData>,
    root: Option<NodeId>,
    /// Deepest node level below the root (root itself is depth 0), written by
    /// `aggregate_sizes`. A rendering-constraints input for the visualizations.
    pub max_depth: u32,
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
            max_depth: 0,
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

    /// Recursively calculates and updates the total size of each directory
    /// node, folds the subtree mtime range, records `max_depth`, and runs the
    /// sibling-offset pass that fills `percent`/`rel_start`. This is an O(N)
    /// post-order traversal operation.
    pub fn aggregate_sizes(&mut self) {
        self.max_depth = 0;
        if let Some(root) = self.root {
            let (size, _, _, _, _) = self.post_order_aggregate(root, 0);
            self.compute_sibling_offsets(root, size, 0);
        }
    }

    fn post_order_aggregate(&mut self, node: NodeId, depth: u32) -> (u64, u64, usize, i64, i64) {
        let mut total_size = 0;
        let mut total_allocated = 0;
        let mut total_count = 0;
        let mut oldest = i64::MAX;
        let mut newest = i64::MIN;

        // Collect children first to appease the borrow checker during
        // mutation; indextree makes the pattern relatively cheap.
        let children: Vec<NodeId> = node.children(&self.arena).collect();

        for child in children {
            let (child_size, child_alloc, child_count, child_old, child_new) =
                self.post_order_aggregate(child, depth + 1);
            total_size += child_size;
            total_allocated += child_alloc;
            total_count += child_count;
            oldest = oldest.min(child_old);
            newest = newest.max(child_new);
        }

        if let Some(data) = self.arena.get_mut(node).map(|n| n.get_mut()) {
            if data.flags.contains(NodeFlags::IS_DIRECTORY)
                || data.flags.contains(NodeFlags::IS_PSEUDO)
            {
                // Directories adopt the accumulated size of their children
                // and count themselves as one item among those children.
                data.size = total_size;
                data.allocated_size = total_allocated;
                data.count = total_count + 1;
                total_count += 1;
                if total_count > 1 {
                    // The children's fold carried real mtimes; an empty
                    // directory keeps its own instead of the i64 sentinels.
                    data.mtime_oldest = if oldest == i64::MAX {
                        data.mtime
                    } else {
                        oldest
                    };
                    data.mtime_newest = if newest == i64::MIN {
                        data.mtime
                    } else {
                        newest
                    };
                }
            } else {
                // Leaves simply contribute their own size and count as one
                data.count = 1;
                total_size += data.size;
                total_allocated += data.allocated_size;
                total_count += data.count;
                data.mtime_oldest = data.mtime;
                data.mtime_newest = data.mtime;
            }
        }

        self.max_depth = self.max_depth.max(depth);

        (
            total_size,
            total_allocated,
            total_count,
            if oldest == i64::MAX {
                self.arena.get(node).map(|n| n.get().mtime).unwrap_or(0)
            } else {
                oldest
            },
            if newest == i64::MIN {
                self.arena.get(node).map(|n| n.get().mtime).unwrap_or(0)
            } else {
                newest
            },
        )
    }

    /// Pre-order pass over the aggregated tree: every node's share of its
    /// parent (`percent`) and the cumulative share of its preceding siblings
    /// (`rel_start`), both in percent of the parent's size. A zero parent
    /// leaves both at zero rather than dividing.
    fn compute_sibling_offsets(&mut self, node: NodeId, parent_size: u64, preceding: u64) {
        let own_size = {
            let data = self.arena.get_mut(node).map(|n| n.get_mut()).unwrap();
            data.percent = percent_of(data.size, parent_size);
            data.rel_start = percent_of(preceding, parent_size);
            data.size
        };

        let children: Vec<NodeId> = node.children(&self.arena).collect();
        let mut cumulative = 0;
        for child in children {
            let child_size = self.arena.get(child).map(|n| n.get().size).unwrap_or(0);
            self.compute_sibling_offsets(child, own_size, cumulative);
            cumulative += child_size;
        }
    }

    /// Removes a node and all of its descendants from the arena, freeing the memory for reuse.
    /// Phase 14/15 scaffold (drill-down rescans): zero callers today.
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
            let dir = tree.add_child(
                current_parent,
                NodeData::new(&format!("dir_{}", i), 0, 0, 0, NodeFlags::IS_DIRECTORY),
            );

            for j in 0..10_000 {
                tree.add_child(
                    dir,
                    NodeData::new(&format!("file_{}", j), 1024, 4096, 0, NodeFlags::empty()),
                );
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

        // Speed is logged, not asserted: debug builds and CI machines vary.
        // Release builds observed < 50ms for this shape.
    }

    #[test]
    fn test_display_stats_percent_rel_start_mtime_range_max_depth() {
        // root/                    mtime 1
        //   big/                   mtime 7
        //     f1      (400)        mtime 30
        //     f2      (100)        mtime 5
        //   small.txt (100)        mtime 20
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("root", 0, 0, 1, NodeFlags::IS_DIRECTORY));
        let big = tree.add_child(root, NodeData::new("big", 0, 0, 7, NodeFlags::IS_DIRECTORY));
        let f1 = tree.add_child(big, NodeData::new("f1", 400, 400, 30, NodeFlags::empty()));
        let f2 = tree.add_child(big, NodeData::new("f2", 100, 100, 5, NodeFlags::empty()));
        let small = tree.add_child(
            root,
            NodeData::new("small.txt", 100, 100, 20, NodeFlags::empty()),
        );

        tree.aggregate_sizes();

        // Totals: root 600, big 500, small.txt 100.
        let close = |a: f32, b: f64| (a as f64 - b).abs() < 1e-3;
        assert!(close(tree.get_data(root).unwrap().percent, 100.0));
        assert!(close(
            tree.get_data(big).unwrap().percent,
            500.0 / 600.0 * 100.0
        ));
        assert!(close(
            tree.get_data(small).unwrap().percent,
            100.0 / 600.0 * 100.0
        ));
        // rel_start: big opens its sibling run at 0; small.txt follows big.
        assert!(close(tree.get_data(big).unwrap().rel_start, 0.0));
        assert!(close(
            tree.get_data(small).unwrap().rel_start,
            500.0 / 600.0 * 100.0
        ));
        // Inside big: f1 400/500 = 80%, f2 follows it at offset 80%.
        assert!(close(tree.get_data(f1).unwrap().percent, 80.0));
        assert!(close(tree.get_data(f2).unwrap().percent, 20.0));
        assert!(close(tree.get_data(f2).unwrap().rel_start, 80.0));

        // Subtree mtime folds: oldest 5 (f2), newest 30 (f1), propagated up.
        assert_eq!(tree.get_data(big).unwrap().mtime_oldest, 5);
        assert_eq!(tree.get_data(big).unwrap().mtime_newest, 30);
        assert_eq!(tree.get_data(root).unwrap().mtime_oldest, 5);
        assert_eq!(tree.get_data(root).unwrap().mtime_newest, 30);
        // Leaves carry their own mtime at both ends.
        assert_eq!(tree.get_data(f1).unwrap().mtime_oldest, 30);
        assert_eq!(tree.get_data(f1).unwrap().mtime_newest, 30);

        // f1/f2 sit two levels under the root.
        assert_eq!(tree.max_depth, 2);

        // An empty directory folds its own mtime rather than sentinels.
        let empty = tree.add_child(
            root,
            NodeData::new("empty", 0, 0, 9, NodeFlags::IS_DIRECTORY),
        );
        tree.aggregate_sizes();
        assert_eq!(tree.get_data(empty).unwrap().mtime_oldest, 9);
        assert_eq!(tree.get_data(empty).unwrap().mtime_newest, 9);
        // And its percent against the unchanged parent total.
        assert!(close(tree.get_data(empty).unwrap().percent, 0.0));
    }

    #[test]
    fn test_aggregation_known_hierarchy() {
        // root/
        //   videos/            = (1000, 2000)
        //     big.mkv          = (900, 1800)
        //     small.mkv        = (100, 200)
        //   docs/              = (100, 260)
        //     nested/          = (30, 120)
        //       a.txt          = (10, 40)
        //       b.txt          = (20, 80)
        //     notes.txt        = (70, 140)
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("root", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let videos = tree.add_child(
            root,
            NodeData::new("videos", 0, 0, 0, NodeFlags::IS_DIRECTORY),
        );
        tree.add_child(
            videos,
            NodeData::new("big.mkv", 900, 1800, 0, NodeFlags::empty()),
        );
        tree.add_child(
            videos,
            NodeData::new("small.mkv", 100, 200, 0, NodeFlags::empty()),
        );

        let docs = tree.add_child(
            root,
            NodeData::new("docs", 0, 0, 0, NodeFlags::IS_DIRECTORY),
        );
        let nested = tree.add_child(
            docs,
            NodeData::new("nested", 0, 0, 0, NodeFlags::IS_DIRECTORY),
        );
        tree.add_child(
            nested,
            NodeData::new("a.txt", 10, 40, 0, NodeFlags::empty()),
        );
        tree.add_child(
            nested,
            NodeData::new("b.txt", 20, 80, 0, NodeFlags::empty()),
        );
        tree.add_child(
            docs,
            NodeData::new("notes.txt", 70, 140, 0, NodeFlags::empty()),
        );

        tree.aggregate_sizes();

        // Intermediate directories carry the sum of everything below them,
        // apparent size and allocated size alike.
        let nested_data = tree.get_data(nested).unwrap();
        assert_eq!(nested_data.size, 30);
        assert_eq!(nested_data.allocated_size, 120);
        let docs_data = tree.get_data(docs).unwrap();
        assert_eq!(docs_data.size, 100);
        assert_eq!(docs_data.allocated_size, 260);
        let videos_data = tree.get_data(videos).unwrap();
        assert_eq!(videos_data.size, 1000);
        assert_eq!(videos_data.allocated_size, 2000);
        // The root carries the whole hierarchy (7 files + 2 directories).
        let root_data = tree.get_data(root).unwrap();
        assert_eq!(root_data.size, 1100);
        assert_eq!(root_data.allocated_size, 2260);
        assert_eq!(root_data.count, 9);
        // Directory counts include the directory itself...
        assert_eq!(tree.get_data(docs).unwrap().count, 5);
        assert_eq!(tree.get_data(nested).unwrap().count, 3);
        assert_eq!(tree.get_data(videos).unwrap().count, 3);
        // ...and each leaf counts as one.
        let big = tree.get_children(videos).next().unwrap();
        assert_eq!(tree.get_data(big).unwrap().size, 900);
        assert_eq!(tree.get_data(big).unwrap().count, 1);
    }
}
