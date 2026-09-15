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
        #[qproperty(QString, initial_path)]
        type ScanBridge = super::ScanBridgeRust;

        #[qinvokable]
        fn start_scan(self: Pin<&mut ScanBridge>, path: QString);

        #[qinvokable]
        fn cancel_scan(self: Pin<&mut ScanBridge>);

        #[qinvokable]
        fn update_metrics(self: Pin<&mut ScanBridge>);

        #[qsignal]
        fn scan_finished(self: Pin<&mut ScanBridge>);
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use topograph_core::FileTree;
use topograph_core::scanner::Scanner;

/// Bumped whenever a scan starts or is cancelled. A scan's worker thread may
/// finish building its tree long after that, so its captured generation is
/// checked before publication: a cancelled scan that drains its channel late,
/// or a superseded scan, must not overwrite the tree slot of a newer scan.
static TREE_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The shared scan-result slot. LazyLock over the Arc keeps the lazy_static
/// dependency out; there is exactly one use site.
pub static LATEST_TREE: LazyLock<Arc<RwLock<Option<FileTree>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Human-readable size in binary units: the largest unit that keeps the
/// value at or above 1, two decimals (976.56 KB, 1.50 MB); byte counts stay
/// whole (512 B, not 512.00 B).
pub(crate) fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

/// Decimal-grouped count: 1234567 -> "1,234,567".
pub(crate) fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    grouped
}

pub struct ScanBridgeRust {
    is_scanning: bool,
    progress_text: QString,
    speed_text: QString,
    current_path: QString,
    /// A path from the command line, surfaced to QML so it can seed the
    /// path field before the first scan.
    initial_path: QString,

    // Internal state
    scanner: Option<Arc<Scanner>>,
    last_files_count: usize,
    last_update_time: Option<std::time::Instant>,
    scan_started: Option<std::time::Instant>,
}

impl Default for ScanBridgeRust {
    fn default() -> Self {
        Self {
            is_scanning: false,
            progress_text: QString::default(),
            speed_text: QString::default(),
            current_path: QString::default(),
            initial_path: QString::from(std::env::args().nth(1).unwrap_or_default().as_str()),
            scanner: None,
            last_files_count: 0,
            last_update_time: None,
            scan_started: None,
        }
    }
}

impl scan_bridge::ScanBridge {
    pub fn start_scan(mut self: Pin<&mut Self>, path: QString) {
        self.as_mut().set_is_scanning(true);
        self.as_mut().set_current_path(path.clone());
        self.as_mut()
            .set_progress_text(QString::from("Starting scan..."));

        // Defensive: stop a superseded walker outright. The UI serializes
        // start/cancel through is_scanning, so no previous scan should
        // exist here, but the generation guard alone would not stop an old
        // walker from continuing to fill its channel.
        if let Some(previous) = &self.rust().scanner {
            previous.cancel();
        }

        let scanner = Arc::new(Scanner::new());
        let mut rust_mut = self.as_mut().rust_mut();
        rust_mut.scanner = Some(scanner.clone());
        rust_mut.last_files_count = 0;
        rust_mut.last_update_time = Some(std::time::Instant::now());
        rust_mut.scan_started = Some(std::time::Instant::now());

        let generation = TREE_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        let rx = scanner.scan_dir(path.to_string());

        let metrics = scanner.metrics.clone();
        std::thread::spawn(move || {
            let mut tree = topograph_core::scanner::build_tree_from_scan(rx);
            tree.aggregate_sizes();

            // A failed scan publishes nothing, so the previously displayed
            // tree survives. The failed flag is stored before the channel
            // closes, and that close is what unblocks this thread, so it is
            // visible here.
            if !metrics.failed.load(Ordering::Relaxed) {
                publish_tree(generation, tree);
            }

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

        let (files, bytes, elapsed, files_diff, now, is_finished, failed, scan_started) = {
            let rust = self.rust();
            if let Some(scanner) = &rust.scanner {
                let metrics = &scanner.metrics;
                let files = metrics.total_files.load(Ordering::Relaxed);
                let bytes = metrics.total_bytes.load(Ordering::Relaxed);
                let is_finished = metrics.is_finished.load(Ordering::Relaxed);
                let failed = metrics.failed.load(Ordering::Relaxed);

                let mut elapsed = 0.0;
                let mut files_diff = 0;
                let now = std::time::Instant::now();

                if let Some(last_time) = rust.last_update_time {
                    elapsed = now.duration_since(last_time).as_secs_f64();
                    if elapsed > 0.1 {
                        files_diff = files.saturating_sub(rust.last_files_count);
                    }
                }
                (
                    files,
                    bytes,
                    elapsed,
                    files_diff,
                    now,
                    is_finished,
                    failed,
                    rust.scan_started,
                )
            } else {
                return;
            }
        };

        if is_finished {
            self.as_mut().set_is_scanning(false);
            self.as_mut().set_speed_text(QString::from(""));
            if failed {
                // The progress line is the error channel, and scan_finished
                // is deliberately NOT emitted: the model keeps the tree it
                // had instead of reloading an empty one.
                let msg = format!("Scan failed: cannot read {}", self.rust().current_path);
                self.as_mut().set_progress_text(QString::from(&msg));
            } else {
                // The summary replaces the bare "Scan complete." with the
                // totals the atomics already carry.
                let total = scan_started
                    .map(|t| t.elapsed().as_secs_f64())
                    .unwrap_or(0.0);
                let summary = format!(
                    "{} files, {} in {:.1}s",
                    format_count(files),
                    format_size(bytes),
                    total
                );
                self.as_mut().set_progress_text(QString::from(&summary));
                // Emitted after the properties settle: the old string-keyed QML
                // refresh read progressText inside onIsScanningChanged, which
                // always fired one write too early and never reloaded the model.
                self.as_mut().scan_finished();
            }
            return;
        }

        let progress = format!("{} files ({})", files, format_size(bytes));
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
    // Load-bearing CXX-Qt 0.6 anti-stripping shim: referencing a bridge
    // method keeps the generated object file linked so QML can find the
    // type. Same pattern as directory_model::force_link; never delete.
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

    #[test]
    fn format_size_uses_the_largest_fitting_unit() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(500 * 1024 * 1024), "500.00 MB");
        assert_eq!(format_size(4 * 1024 * 1024 * 1024), "4.00 GB");
        assert_eq!(format_size(5 * 1024_u64.pow(4)), "5.00 TB");
        // Past the last unit, stop dividing.
        assert_eq!(format_size(16 * 1024_u64.pow(4)), "16.00 TB");
    }

    #[test]
    fn format_count_groups_thousands() {
        assert_eq!(format_count(7), "7");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(100_000), "100,000");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }
}
