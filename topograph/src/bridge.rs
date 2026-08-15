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
use topograph_core::scanner::Scanner;
use std::sync::atomic::Ordering;

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
        self.as_mut().set_progress_text(QString::from("Starting scan..."));
        
        let scanner = Arc::new(Scanner::new());
        let mut rust_mut = self.as_mut().rust_mut();
        rust_mut.scanner = Some(scanner.clone());
        rust_mut.last_files_count = 0;
        rust_mut.last_update_time = Some(std::time::Instant::now());

        let rx = scanner.scan_dir(path.to_string());
        
        let metrics = scanner.metrics.clone();
        std::thread::spawn(move || {
            let mut tree = topograph_core::scanner::build_tree_from_scan(rx);
            tree.aggregate_sizes();
            
            if let Ok(mut lock) = LATEST_TREE.write() {
                *lock = Some(tree);
            }
            
            metrics.is_finished.store(true, Ordering::Relaxed);
        });
    }

    pub fn cancel_scan(mut self: Pin<&mut Self>) {
        if let Some(scanner) = &self.rust().scanner {
            scanner.cancel();
        }
        self.as_mut().set_is_scanning(false);
        self.as_mut().set_progress_text(QString::from("Scan cancelled."));
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
            self.as_mut().set_progress_text(QString::from("Scan complete."));
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

pub fn force_link() {
    let _ = scan_bridge::ScanBridge::start_scan as *const ();
}
