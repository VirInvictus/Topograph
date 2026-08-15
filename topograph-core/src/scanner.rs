use jwalk::WalkDir;
use std::path::{Path, PathBuf};
use crossbeam_channel::{Sender, Receiver, bounded};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::{NodeData, NodeFlags, FileTree};
use indextree::NodeId;
use std::collections::HashMap;

pub struct ScanResult {
    pub path: PathBuf,
    pub parent_path: Option<PathBuf>,
    pub data: NodeData,
}

pub struct Scanner {
    cancel_token: Arc<AtomicBool>,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.store(true, Ordering::SeqCst);
    }

    pub fn scan_dir<P: AsRef<Path>>(&self, root: P) -> Receiver<ScanResult> {
        let (tx, rx) = bounded(10_000);
        let root_path = root.as_ref().to_path_buf();
        let cancel_token = self.cancel_token.clone();

        std::thread::spawn(move || {
            // jwalk parallel work-stealing traversal.
            // By default, skip_hidden is false and symlinks are not followed.
            for entry in WalkDir::new(&root_path).skip_hidden(false) {
                if cancel_token.load(Ordering::Relaxed) {
                    break;
                }

                if let Ok(dir_entry) = entry {
                    let path = dir_entry.path();
                    let parent_path = path.parent().map(|p| p.to_path_buf());
                    
                    let mut flags = NodeFlags::empty();
                    let file_type = dir_entry.file_type();
                    if file_type.is_dir() {
                        flags.insert(NodeFlags::IS_DIRECTORY);
                    } else if file_type.is_symlink() {
                        flags.insert(NodeFlags::IS_SYMLINK);
                    }

                    // Metadata fetch handles POSIX stat, falling back cleanly on EACCES
                    let (size, allocated_size, mtime) = if let Ok(metadata) = dir_entry.metadata() {
                        use std::os::unix::fs::MetadataExt;
                        (
                            metadata.len(),
                            metadata.blocks() * 512, // Standard POSIX blocks are 512 bytes
                            metadata.mtime(),
                        )
                    } else {
                        (0, 0, 0)
                    };

                    let name = dir_entry.file_name().to_string_lossy().to_string();

                    let result = ScanResult {
                        path,
                        parent_path,
                        data: NodeData::new(&name, size, allocated_size, mtime, flags),
                    };

                    if tx.send(result).is_err() {
                        break; // Receiver dropped, stop scanning
                    }
                }
            }
        });

        rx
    }
}

/// Helper to consume a ScanResult receiver and build a FileTree safely.
/// `jwalk` guarantees parents are yielded before children, allowing $O(1)$ Hashmap mapping.
pub fn build_tree_from_scan(rx: Receiver<ScanResult>) -> FileTree {
    let mut tree = FileTree::new();
    let mut path_to_node: HashMap<PathBuf, NodeId> = HashMap::new();

    for result in rx {
        let node_id = if let Some(parent_path) = result.parent_path.as_ref() {
            if let Some(&parent_id) = path_to_node.get(parent_path) {
                tree.add_child(parent_id, result.data)
            } else {
                // E.g., The root path of the scan itself
                tree.set_root(result.data)
            }
        } else {
            tree.set_root(result.data)
        };
        
        
        path_to_node.insert(result.path, node_id);
    }
    tree
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_scanner_on_src() {
        let scanner = Scanner::new();
        let start = Instant::now();
        // Scan our own src directory
        let rx = scanner.scan_dir("src");
        let mut tree = build_tree_from_scan(rx);
        tree.aggregate_sizes();
        
        let elapsed = start.elapsed();
        println!("Scanned and built tree for 'src' in {:?}", elapsed);
        
        assert!(tree.get_data(tree.root.unwrap()).is_some());
    }
}
