#[cxx_qt::bridge]
pub mod scan_bridge {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, is_scanning)]
        #[qproperty(QString, progress_text)]
        #[qproperty(QString, speed_text)]
        #[qproperty(QString, current_path)]
        type ScanBridge = super::ScanBridgeRust;

        #[qinvokable]
        fn start_scan(self: Pin<&mut ScanBridge>, path: QString);

        #[qinvokable]
        fn cancel_scan(self: Pin<&mut ScanBridge>);

        #[qinvokable]
        fn update_metrics(self: Pin<&mut ScanBridge>);
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use topograph_core::scanner::Scanner;

/// Bumped whenever a scan starts or is cancelled. A scan's worker thread may
/// finish building its tree long after that, so its captured generation is
/// checked before publication: a cancelled scan that drains its channel late,
/// or a superseded scan, must not overwrite the tree slot of a newer scan.
static TREE_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub struct ScanBridgeRust {
    is_scanning: bool,
    progress_text: QString,
    speed_text: QString,
    current_path: QString,

    // Internal state
    scanner: Option<Arc<Scanner>>,
    last_files_count: usize,
    last_update_time: Option<std::time::Instant>,
}

use std::sync::RwLock;
use topograph_core::FileTree;

lazy_static::lazy_static! {
    pub static ref LATEST_TREE: Arc<RwLock<Option<FileTree>>> = Arc::new(RwLock::new(None));
}

impl scan_bridge::ScanBridge {
    pub fn start_scan(mut self: Pin<&mut Self>, path: QString) {
        self.as_mut().set_is_scanning(true);
        self.as_mut().set_current_path(path.clone());
        self.as_mut()
            .set_progress_text(QString::from("Starting scan..."));

        let scanner = Arc::new(Scanner::new());
        let mut rust_mut = self.as_mut().rust_mut();
        rust_mut.scanner = Some(scanner.clone());
        rust_mut.last_files_count = 0;
        rust_mut.last_update_time = Some(std::time::Instant::now());

        let generation = TREE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        let rx = scanner.scan_dir(path.to_string());

        let metrics = scanner.metrics.clone();
        std::thread::spawn(move || {
            let mut tree = topograph_core::scanner::build_tree_from_scan(rx);
            tree.aggregate_sizes();

            publish_tree(generation, tree);

            metrics.is_finished.store(true, Ordering::Relaxed);
        });
    }

    pub fn cancel_scan(mut self: Pin<&mut Self>) {
        // Invalidate the in-flight worker's publish along with stopping it.
        TREE_GENERATION.fetch_add(1, Ordering::SeqCst);
        if let Some(scanner) = &self.rust().scanner {
            scanner.cancel();
        }
        self.as_mut().set_is_scanning(false);
        self.as_mut()
            .set_progress_text(QString::from("Scan cancelled."));
        self.as_mut().set_speed_text(QString::from(""));
    }

    pub fn update_metrics(mut self: Pin<&mut Self>) {
        if !self.rust().is_scanning {
            return;
        }

        let (files, bytes, elapsed, files_diff, now, is_finished) = {
            let rust = self.rust();
            if let Some(scanner) = &rust.scanner {
                let metrics = &scanner.metrics;
                let files = metrics.total_files.load(Ordering::Relaxed);
                let bytes = metrics.total_bytes.load(Ordering::Relaxed);
                let is_finished = metrics.is_finished.load(Ordering::Relaxed);

                let mut elapsed = 0.0;
                let mut files_diff = 0;
                let now = std::time::Instant::now();

                if let Some(last_time) = rust.last_update_time {
                    elapsed = now.duration_since(last_time).as_secs_f64();
                    if elapsed > 0.1 {
                        files_diff = files.saturating_sub(rust.last_files_count);
                    }
                }
                (files, bytes, elapsed, files_diff, now, is_finished)
            } else {
                return;
            }
        };

        if is_finished {
            self.as_mut().set_is_scanning(false);
            self.as_mut()
                .set_progress_text(QString::from("Scan complete."));
            self.as_mut().set_speed_text(QString::from(""));
            return;
        }

        let mb = bytes as f64 / (1024.0 * 1024.0);
        let progress = format!("{} files ({:.2} MB)", files, mb);
        self.as_mut().set_progress_text(QString::from(&progress));

        if elapsed > 0.1 {
            let speed = (files_diff as f64 / elapsed) as usize;
            let speed_str = format!("{} files/sec", speed);
            self.as_mut().set_speed_text(QString::from(&speed_str));

            let mut rust_mut = self.as_mut().rust_mut();
            rust_mut.last_files_count = files;
            rust_mut.last_update_time = Some(now);
        }
    }
}

/// Publishes a fully built tree to the shared slot, unless a newer scan has
/// started (or this one was cancelled) since it began. Returns whether the
/// tree was published.
fn publish_tree(generation: u64, tree: FileTree) -> bool {
    if TREE_GENERATION.load(Ordering::SeqCst) != generation {
        return false;
    }
    match LATEST_TREE.write() {
        Ok(mut lock) => {
            *lock = Some(tree);
            true
        }
        Err(_) => false,
    }
}

pub fn force_link() {
    let _ = scan_bridge::ScanBridge::start_scan as *const ();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use topograph_core::{NodeData, NodeFlags};

    /// The tree slot and generation counter are process globals, so tests
    /// that touch them serialize here.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn leaf_tree(name: &str) -> FileTree {
        let mut tree = FileTree::new();
        tree.set_root(NodeData::new(name, 1, 1, 0, NodeFlags::empty()));
        tree
    }

    #[test]
    fn stale_scan_tree_is_not_published() {
        let _guard = TEST_LOCK.lock().unwrap();
        *LATEST_TREE.write().unwrap() = None;
        let current = TREE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        let stale = current - 1;

        // A cancelled or superseded scan finishing late must not publish.
        assert!(!publish_tree(stale, leaf_tree("stale")));
        assert!(LATEST_TREE.read().unwrap().is_none());

        // The current generation publishes normally.
        assert!(publish_tree(current, leaf_tree("fresh")));
        {
            let lock = LATEST_TREE.read().unwrap();
            let tree = lock.as_ref().unwrap();
            let root = tree.get_root().unwrap();
            assert_eq!(tree.get_data(root).unwrap().name.as_ref(), "fresh");
        }

        *LATEST_TREE.write().unwrap() = None;
    }
}
