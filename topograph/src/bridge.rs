#[cxx_qt::bridge]
pub mod scan_bridge {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("treemap_view.h");
        fn topograph_treemap_force_link();

        include!("selftest_support.h");
        fn topograph_selftest_capture(
            out_dir: &QString,
            scan_path: &QString,
            pixel_checks: bool,
        ) -> i32;
    }

    extern "Rust" {
        // Treemap vertex plumbing for the C++ QSGGeometryNode renderer: the
        // buffer is built into a process-global slot (same pattern as
        // LATEST_TREE) and copied out in one call, so no per-vertex FFI.
        fn treemap_rebuild(
            width: f32,
            height: f32,
            cushion_height: f32,
            ambient: f32,
            light_angle: f32,
            padding: f32,
        ) -> usize;
        unsafe fn treemap_copy_vertices(out: *mut u8, len: usize);
        fn treemap_tile_count() -> usize;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, is_scanning)]
        #[qproperty(QString, progress_text)]
        #[qproperty(QString, speed_text)]
        #[qproperty(QString, current_path)]
        #[qproperty(QString, initial_path)]
        #[qproperty(bool, cross_filesystems)]
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
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use topograph_core::FileTree;
use topograph_core::scanner::Scanner;
use topograph_core::treemap::{self, TreemapOptions, TreemapVertex};

/// Bumped whenever a scan starts or is cancelled. A scan's worker thread may
/// finish building its tree long after that, so its captured generation is
/// checked before publication: a cancelled scan that drains its channel late,
/// or a superseded scan, must not overwrite the tree slot of a newer scan.
static TREE_GENERATION: AtomicU64 = AtomicU64::new(0);

/// The shared scan-result slot. LazyLock over the Arc keeps the lazy_static
/// dependency out; there is exactly one use site.
pub static LATEST_TREE: LazyLock<Arc<RwLock<Option<FileTree>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// The packed treemap vertex buffer handed to the C++ renderer: interleaved
/// little-endian x,y (f32) + rgba (u8), 12 bytes per vertex, the exact
/// layout of Qt's colored-point geometry. Single-consumer assumption, like
/// LATEST_TREE: one TreemapView interleaves rebuild/copy pairs safely, a
/// second instance would need its own plumbing.
pub static TREEMAP_VERTICES: LazyLock<RwLock<Vec<u8>>> = LazyLock::new(|| RwLock::new(Vec::new()));

/// Tile (laid rectangle) count from the last rebuild, for the UI counter;
/// distinct from the vertex count (each tile emits a grid of triangles).
pub static TREEMAP_TILE_COUNT: AtomicUsize = AtomicUsize::new(0);

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

/// Packs vertices into the renderer's byte format: little-endian x/y floats
/// followed by straight RGBA bytes, 12 bytes per vertex.
fn pack_vertices(vertices: &[TreemapVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * 12);
    for v in vertices {
        bytes.extend_from_slice(&v.x.to_le_bytes());
        bytes.extend_from_slice(&v.y.to_le_bytes());
        bytes.extend_from_slice(&v.color);
    }
    bytes
}

/// Builds the treemap for the currently displayed tree at `width` x
/// `height`, stores it in the shared slot, and returns the byte count. An
/// absent tree stores an empty buffer (the renderer shows nothing). Reading
/// LATEST_TREE holds its read lock only for the build, which takes
/// single-digit milliseconds at full HD sizes.
fn treemap_rebuild(
    width: f32,
    height: f32,
    cushion_height: f32,
    ambient: f32,
    light_angle: f32,
    padding: f32,
) -> usize {
    let options = TreemapOptions {
        cushion_height,
        ambient,
        light_angle,
        padding,
        ..TreemapOptions::default()
    };
    let mut bytes = Vec::new();
    let mut tiles = 0usize;
    if let Ok(lock) = LATEST_TREE.read()
        && let Some(tree) = lock.as_ref()
        && let Some(root) = tree.get_root()
    {
        let (verts, laid) = treemap::build_vertices_counted(tree, root, width, height, &options);
        tiles = laid;
        bytes = pack_vertices(&verts);
    }
    let len = bytes.len();
    if let Ok(mut slot) = TREEMAP_VERTICES.write() {
        *slot = bytes;
    }
    TREEMAP_TILE_COUNT.store(tiles, Ordering::Relaxed);
    len
}

/// Tile count from the last rebuild, for the UI counter.
fn treemap_tile_count() -> usize {
    TREEMAP_TILE_COUNT.load(Ordering::Relaxed)
}

/// Copies the stored vertex buffer to the renderer's memory. The caller
/// allocates from the length `treemap_rebuild` returned; copying is bounded
/// to whichever side is shorter, so a rebuild racing between the two calls
/// cannot write past the destination.
fn treemap_copy_vertices(out: *mut u8, len: usize) {
    if let Ok(lock) = TREEMAP_VERTICES.read() {
        let n = len.min(lock.len());
        if n > 0 {
            unsafe { std::ptr::copy_nonoverlapping(lock.as_ptr(), out, n) };
        }
    }
}

/// The completion summary: entry count, apparent vs allocated totals, scan
/// duration, and the hardlink-dedup savings when the tree had multi-linked
/// files.
pub(crate) fn scan_summary(
    files: usize,
    bytes: u64,
    allocated: u64,
    saved: u64,
    secs: f64,
) -> String {
    let mut summary = format!(
        "{} files, {} apparent / {} on disk in {:.1}s",
        format_count(files),
        format_size(bytes),
        format_size(allocated),
        secs
    );
    if saved > 0 {
        summary.push_str(&format!("; dedup saved {}", format_size(saved)));
    }
    summary
}

pub struct ScanBridgeRust {
    is_scanning: bool,
    progress_text: QString,
    speed_text: QString,
    current_path: QString,
    /// A path from the command line, surfaced to QML so it can seed the
    /// path field before the first scan.
    initial_path: QString,
    /// Opt-in mount-boundary crossing (the Phase 3 toggle): off by default,
    /// so a scan of / stays on the root filesystem. With it on, the walk
    /// descends into every mounted filesystem it meets.
    cross_filesystems: bool,

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
            cross_filesystems: false,
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

        // The mount-boundary toggle is read once per scan; flipping it
        // mid-scan takes effect on the next scan.
        let mut scanner = Scanner::new();
        scanner.cross_filesystems = self.rust().cross_filesystems;
        let scanner = Arc::new(scanner);
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

        let (
            files,
            bytes,
            allocated,
            saved,
            elapsed,
            files_diff,
            now,
            is_finished,
            failed,
            scan_started,
        ) = {
            let rust = self.rust();
            if let Some(scanner) = &rust.scanner {
                let metrics = &scanner.metrics;
                let files = metrics.total_files.load(Ordering::Relaxed);
                let bytes = metrics.total_bytes.load(Ordering::Relaxed);
                let allocated = metrics.total_allocated.load(Ordering::Relaxed);
                let saved = metrics.dedup_saved_bytes.load(Ordering::Relaxed);
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
                    allocated,
                    saved,
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
                let summary = scan_summary(files, bytes, allocated, saved, total);
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
    // Same job for the hand-written C++ TreemapView: it registers its QML
    // type from a static initializer, so without this call the linker drops
    // its object file and the com.topograph.treemap import dies at runtime.
    scan_bridge::topograph_treemap_force_link();
    // The selftest entry point lives in the topograph-selftest bin; the
    // main bin never calls it, so reference it here to keep the bridge's
    // declaration compiled in both binaries.
    let _: fn(&QString, &QString, bool) -> i32 = scan_bridge::topograph_selftest_capture;
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

    #[test]
    fn scan_summary_shows_both_totals_and_dedup_savings() {
        // Apparent and on-disk totals, no dedup clause without hardlinks.
        assert_eq!(
            scan_summary(5935, 179_472_793, 214_643_507, 0, 0.42),
            "5,935 files, 171.16 MB apparent / 204.70 MB on disk in 0.4s"
        );
        // Savings append when duplicate links were encountered.
        assert_eq!(
            scan_summary(10, 2048, 8192, 1024, 1.0),
            "10 files, 2.00 KB apparent / 8.00 KB on disk in 1.0s; dedup saved 1.00 KB"
        );
    }

    #[test]
    fn pack_vertices_matches_the_colored_point_layout() {
        let verts = vec![TreemapVertex {
            x: 1.5,
            y: -2.25,
            color: [1, 2, 3, 255],
        }];
        let bytes = pack_vertices(&verts);
        assert_eq!(bytes.len(), 12);
        assert_eq!(&bytes[0..4], &1.5f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &(-2.25f32).to_le_bytes());
        assert_eq!(&bytes[8..12], &[1, 2, 3, 255]);
    }

    #[test]
    fn treemap_slot_round_trips_and_empty_tree_clears_it() {
        let _guard = TEST_LOCK.lock().unwrap();

        // A published tree lays tiles into the slot.
        *LATEST_TREE.write().unwrap() = Some(fixture_tree());
        let len = treemap_rebuild(400.0, 300.0, 0.7, 0.55, 3.927, 3.0);
        assert!(len > 0);
        assert_eq!(len % 12, 0);
        // Four laid tiles (a, sub, b, c), each a grid of triangles: the
        // counter reports tiles, not vertices.
        assert_eq!(treemap_tile_count(), 4);
        {
            let slot = TREEMAP_VERTICES.read().unwrap();
            assert_eq!(slot.len(), len);
            // Opaque alpha on the first vertex.
            assert_eq!(slot[11], 255);
        }
        let mut out = vec![0u8; len];
        treemap_copy_vertices(out.as_mut_ptr(), len);
        assert_eq!(out, *TREEMAP_VERTICES.read().unwrap().as_slice());

        // No tree (a failed scan never publishes): the slot empties and the
        // renderer draws nothing.
        *LATEST_TREE.write().unwrap() = None;
        let len = treemap_rebuild(400.0, 300.0, 0.7, 0.55, 3.927, 3.0);
        assert_eq!(len, 0);
        assert!(TREEMAP_VERTICES.read().unwrap().is_empty());
        assert_eq!(treemap_tile_count(), 0);

        // A short destination buffer copies only what fits.
        *LATEST_TREE.write().unwrap() = Some(fixture_tree());
        treemap_rebuild(400.0, 300.0, 0.7, 0.55, 3.927, 3.0);
        let mut small = vec![0u8; 12];
        treemap_copy_vertices(small.as_mut_ptr(), 12);
        assert_eq!(&small, &TREEMAP_VERTICES.read().unwrap().as_slice()[..12]);

        *LATEST_TREE.write().unwrap() = None;
        *TREEMAP_VERTICES.write().unwrap() = Vec::new();
    }

    /// root/ with two files and a subdirectory, aggregated.
    fn fixture_tree() -> FileTree {
        use topograph_core::{NodeData, NodeFlags};
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("root", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        tree.add_child(root, NodeData::new("a", 600, 600, 0, NodeFlags::empty()));
        let sub = tree.add_child(root, NodeData::new("sub", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        tree.add_child(sub, NodeData::new("b", 300, 300, 0, NodeFlags::empty()));
        tree.add_child(sub, NodeData::new("c", 100, 100, 0, NodeFlags::empty()));
        tree.aggregate_sizes();
        tree
    }
}
