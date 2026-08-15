use jwalk::WalkDir;
use std::path::{Path, PathBuf};
use crossbeam_channel::{Sender, Receiver, bounded};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::{NodeData, NodeFlags, FileTree};
use indextree::NodeId;
use std::collections::HashMap;
use dashmap::DashSet;
use std::os::unix::fs::MetadataExt;

pub struct ScanResult {
    pub path: PathBuf,
    pub parent_path: Option<PathBuf>,
    pub data: NodeData,
}

pub struct Scanner {
    cancel_token: Arc<AtomicBool>,
    pub cross_filesystems: bool,
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
            cross_filesystems: false, // Default to strict boundaries
        }
    }

    pub fn cancel(&self) {
        self.cancel_token.store(true, Ordering::SeqCst);
    }

    pub fn scan_dir<P: AsRef<Path>>(&self, root: P) -> Receiver<ScanResult> {
        let (tx, rx) = bounded(10_000);
        let root_path = root.as_ref().to_path_buf();
        let cancel_token = self.cancel_token.clone();
        let cross_fs = self.cross_filesystems;

        std::thread::spawn(move || {
            let root_metadata = std::fs::symlink_metadata(&root_path).ok();
            let root_dev = root_metadata.map(|m| m.dev()).unwrap_or(0);
            
            // DashSet tracks (dev, inode) for fast-path O(1) hardlink deduplication across threads
            let seen_inodes = Arc::new(DashSet::<(u64, u64)>::new());

            let walker = WalkDir::new(&root_path)
                .skip_hidden(false)
                .process_read_dir(move |_depth, _path, _state, children| {
                    if !cross_fs && root_dev != 0 {
                        // Dynamically prune directories that cross into another filesystem 
                        // (e.g. /proc, /sys, or mounted drives)
                        children.retain(|dir_entry_result| {
                            if let Ok(dir_entry) = dir_entry_result {
                                if let Ok(metadata) = dir_entry.metadata() {
                                    if metadata.dev() != root_dev {
                                        return false; // Skip traversing this branch
                                    }
                                }
                            }
                            true
                        });
                    }
                });

            for entry in walker {
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
                    let (mut size, mut allocated_size, mtime) = if let Ok(metadata) = dir_entry.metadata() {
                        let dev = metadata.dev();
                        let inode = metadata.ino();
                        let nlink = metadata.nlink();
                        let mut s = metadata.len();
                        let mut alloc = metadata.blocks() * 512;

                        // Fast-path hardlink deduplication: 
                        // We only care about checking the concurrent HashSet if st_nlink > 1
                        if !file_type.is_dir() && nlink > 1 {
                            // If insert returns false, the (dev, ino) is already known!
                            if !seen_inodes.insert((dev, inode)) {
                                s = 0;
                                alloc = 0;
                                flags.insert(NodeFlags::IS_HARDLINK_DUPE);
                            }
                        }

                        (s, alloc, metadata.mtime())
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

pub fn build_tree_from_scan(rx: Receiver<ScanResult>) -> FileTree {
    let mut tree = FileTree::new();
    let mut path_to_node: HashMap<PathBuf, NodeId> = HashMap::new();

    for result in rx {
        let node_id = if let Some(parent_path) = result.parent_path.as_ref() {
            if let Some(&parent_id) = path_to_node.get(parent_path) {
                tree.add_child(parent_id, result.data)
            } else {
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
    use std::fs::{self, File};
    use tempfile::tempdir;

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

    #[test]
    fn test_hardlink_deduplication() {
        let dir = tempdir().unwrap();
        let file1_path = dir.path().join("file1.txt");
        let file2_path = dir.path().join("file2_link.txt");

        // Create a 1024-byte file
        fs::write(&file1_path, vec![0u8; 1024]).unwrap();
        // Hardlink it
        fs::hard_link(&file1_path, &file2_path).unwrap();

        let scanner = Scanner::new();
        let rx = scanner.scan_dir(dir.path());
        let mut tree = build_tree_from_scan(rx);
        tree.aggregate_sizes();

        let root_data = tree.get_data(tree.root.unwrap()).unwrap();
        // Even though there are two 1024-byte files, because of hardlink dedup, 
        // the size should exactly equal 1024.
        assert_eq!(root_data.size, 1024);
    }
}
