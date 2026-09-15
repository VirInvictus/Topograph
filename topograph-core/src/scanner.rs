use crate::{FileTree, NodeData, NodeFlags};
use crossbeam_channel::{Receiver, bounded};
use dashmap::DashSet;
use indextree::NodeId;
use jwalk::WalkDirGeneric;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

pub struct ScanResult {
    pub path: PathBuf,
    pub parent_path: Option<PathBuf>,
    pub data: NodeData,
}

#[derive(Default)]
pub struct ScanMetrics {
    pub total_files: AtomicUsize,
    pub total_bytes: AtomicU64,
    pub is_finished: AtomicBool,
    /// The scan failed (root missing or unreadable): the bridge surfaces it
    /// as an error instead of publishing an empty tree as success.
    pub failed: AtomicBool,
}

pub struct Scanner {
    cancel_token: Arc<AtomicBool>,
    pub cross_filesystems: bool,
    pub metrics: Arc<ScanMetrics>,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-entry stat results, captured once in `process_read_dir` so the main
/// loop never re-stats: jwalk does not cache metadata between calls, and a
/// second lstat per entry halves metadata throughput.
#[derive(Clone, Copy, Default, Debug)]
struct EntryMeta {
    ok: bool,
    dev: u64,
    ino: u64,
    nlink: u64,
    size: u64,
    allocated: u64,
    mtime: i64,
}

impl EntryMeta {
    fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        EntryMeta {
            ok: true,
            dev: metadata.dev(),
            ino: metadata.ino(),
            nlink: metadata.nlink(),
            size: metadata.len(),
            allocated: metadata.blocks() * 512,
            mtime: metadata.mtime(),
        }
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            cancel_token: Arc::new(AtomicBool::new(false)),
            cross_filesystems: false, // Default to strict boundaries
            metrics: Arc::new(ScanMetrics::default()),
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
        let metrics = self.metrics.clone();

        std::thread::spawn(move || {
            // A root that cannot be stat'ed (nonexistent, or its parent
            // unreadable) is a failed scan, not an empty one.
            let Ok(root_metadata) = std::fs::symlink_metadata(&root_path) else {
                metrics.failed.store(true, Ordering::Relaxed);
                return;
            };
            let root_dev = root_metadata.dev();

            // DashSet tracks (dev, inode) for fast-path O(1) hardlink deduplication across threads
            let seen_inodes = Arc::new(DashSet::<(u64, u64)>::new());

            let walker = WalkDirGeneric::<((), EntryMeta)>::new(&root_path)
                .skip_hidden(false)
                .process_read_dir(move |_depth, _path, _state, children| {
                    // One lstat per entry here, stashed into client_state for
                    // the main loop (jwalk does not cache metadata between
                    // calls). Any stat error keeps the entry; its sizes zero
                    // out below.
                    for dir_entry in children.iter_mut().flatten() {
                        if let Ok(metadata) = dir_entry.metadata() {
                            dir_entry.client_state = EntryMeta::from_metadata(&metadata);
                        }
                    }
                    if !cross_fs && root_dev != 0 {
                        // Dynamically prune directories that cross into another
                        // filesystem (e.g. /proc, /sys, or mounted drives),
                        // deciding on the stashed dev.
                        children.retain(|dir_entry_result| match dir_entry_result {
                            Ok(dir_entry) => {
                                let meta = dir_entry.client_state;
                                !meta.ok || meta.dev == root_dev
                            }
                            Err(_) => true,
                        });
                    }
                });

            let mut saw_entry = false;
            for entry in walker {
                if cancel_token.load(Ordering::Relaxed) {
                    break;
                }

                if let Ok(dir_entry) = entry {
                    saw_entry = true;
                    let path = dir_entry.path();
                    let parent_path = path.parent().map(|p| p.to_path_buf());

                    let mut flags = NodeFlags::empty();
                    let file_type = dir_entry.file_type();
                    if file_type.is_dir() {
                        flags.insert(NodeFlags::IS_DIRECTORY);
                    } else if file_type.is_symlink() {
                        flags.insert(NodeFlags::IS_SYMLINK);
                    }

                    // The stash from process_read_dir covers everything but
                    // the root entry (which never passes through that
                    // closure); the fallback re-stats just those. Any stat
                    // error silently zeroes the entry's sizes.
                    let meta = dir_entry.client_state;
                    let meta = if meta.ok {
                        meta
                    } else {
                        dir_entry
                            .metadata()
                            .map(|m| EntryMeta::from_metadata(&m))
                            .unwrap_or_default()
                    };

                    let (size, allocated_size, mtime) = if !file_type.is_dir() && meta.ok && meta.nlink > 1
                            // If insert returns false, the (dev, ino) is already known!
                            && !seen_inodes.insert((meta.dev, meta.ino))
                    {
                        flags.insert(NodeFlags::IS_HARDLINK_DUPE);
                        (0, 0, meta.mtime)
                    } else {
                        (meta.size, meta.allocated, meta.mtime)
                    };

                    let name = dir_entry.file_name().to_string_lossy().to_string();

                    metrics.total_files.fetch_add(1, Ordering::Relaxed);
                    metrics.total_bytes.fetch_add(size, Ordering::Relaxed);

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

            // A root that stats but yields nothing could not be read (EACCES
            // on the root itself); an existing empty directory still yields
            // its own entry. Cancelled scans break out early and must not
            // report failure.
            if !saw_entry && !cancel_token.load(Ordering::Relaxed) {
                metrics.failed.store(true, Ordering::Relaxed);
            }
        });

        rx
    }
}

/// Builds the arena from a scan stream. Relies on jwalk's strict pre-order
/// DFS: a parent is always yielded before its children, so `path_to_node`
/// already holds the directory id every child needs when it arrives. An
/// entry whose parent never appeared adopts as a fresh root (in practice
/// only the scan root itself, whose parent lies outside the scan).
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
    use std::fs;
    use std::time::Instant;
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

    #[test]
    fn test_failed_root_sets_the_error_flag() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let scanner = Scanner::new();
        let metrics = scanner.metrics.clone();
        let rx = scanner.scan_dir(&missing);
        // Draining the channel waits for the worker to finish; the send end
        // closes when the failed-scan early return drops it.
        let results: Vec<ScanResult> = rx.iter().collect();

        assert!(metrics.failed.load(Ordering::Relaxed));
        assert!(results.is_empty());
        // An empty tree with no root: the bridge must not publish this over
        // a previously displayed tree.
        let tree = build_tree_from_scan(rx);
        assert!(tree.get_root().is_none());
    }

    #[test]
    fn test_empty_directory_is_not_a_failure() {
        let dir = tempdir().unwrap();

        let scanner = Scanner::new();
        let metrics = scanner.metrics.clone();
        let rx = scanner.scan_dir(dir.path());
        let mut tree = build_tree_from_scan(rx);
        tree.aggregate_sizes();

        assert!(!metrics.failed.load(Ordering::Relaxed));
        // The directory's own entry still yields, so the tree has a root.
        assert!(tree.get_root().is_some());
    }
}
