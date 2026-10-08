//! Headless GUI selftest: loads the real `main.qml` on the offscreen
//! platform, drives it through property writes and method invokes (no
//! synthetic input, no compositor), and captures frames via
//! `QQuickWindow::grabWindow`. The whole walk lives in
//! `selftest_support.cpp`; this entry point builds the scan fixture and
//! owns the graphics environment.
//!
//! Graphics negotiation: the default run tries the GPU path (RHI over the
//! offscreen platform, which works wherever a headless EGL context exists,
//! e.g. Mesa surfaceless + llvmpipe) because the software scene graph
//! cannot draw the treemap's raw QSGGeometryNode. If the GPU path renders
//! nothing at all, the process re-execs itself once on the software
//! backend, where the treemap's pixel checks print SKIP lines instead of
//! silently passing. Overrides: TOPOGRAPH_SELFTEST_GL=1 (no fallback),
//! TOPOGRAPH_SELFTEST_SOFTWARE=1 (software directly), plus the usual
//! QT_QPA_PLATFORM/QT_QUICK_BACKEND.
//!
//! Run: `cargo run -p topograph --bin topograph-selftest [output-dir]`
//! (captures land in the output dir, defaulting to /tmp/topograph-selftest;
//! the exit code is zero only when every requested step passed).

mod bridge;
mod directory_model;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let force_software = std::env::var_os("TOPOGRAPH_SELFTEST_SOFTWARE").is_some();
    let no_fallback = std::env::var_os("TOPOGRAPH_SELFTEST_GL").is_some();

    // Safety: single-threaded startup before any threads exist.
    unsafe {
        if std::env::var_os("QT_QPA_PLATFORM").is_none() {
            std::env::set_var("QT_QPA_PLATFORM", "offscreen");
        }
        if force_software && std::env::var_os("QT_QUICK_BACKEND").is_none() {
            std::env::set_var("QT_QUICK_BACKEND", "software");
        }
    }

    let out_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/topograph-selftest".to_string());
    let fixture = build_fixture();

    let code = run(&out_dir, &fixture, !force_software);

    // Retry triggers: no window (1), nothing renderable at all (2), or the
    // renderer came up as software when GL pixel checks were requested (14;
    // Qt can fall back to the software scene graph on its own when the
    // platform has no usable EGL, as on the CI runner).
    if (code == 1 || code == 2 || code == 14) && !no_fallback && !force_software {
        // Retry once on the software backend (offscreen, forced) so a
        // machine without headless EGL still gets the non-pixel checks.
        eprintln!("GL path unavailable (code {code}); retrying on the software renderer");
        let exe = std::env::current_exe().expect("selftest exe path");
        let status = std::process::Command::new(exe)
            .args(std::env::args().skip(1))
            .env("TOPOGRAPH_SELFTEST_SOFTWARE", "1")
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("QT_QUICK_BACKEND", "software")
            .status()
            .expect("selftest re-exec");
        std::process::exit(status.code().unwrap_or(1));
    }

    if code == 0 {
        println!("captures in {out_dir}");
    } else {
        eprintln!("selftest failed with code {code}; captures in {out_dir}");
    }
    std::process::exit(code);
}

fn run(out_dir: &str, fixture: &str, pixel_checks: bool) -> i32 {
    // Keep the generated QML types linked, exactly like main.rs.
    bridge::force_link();
    directory_model::force_link();

    let app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    let code = if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/com/topograph/qml/main.qml"));
        bridge::scan_bridge::topograph_selftest_capture(
            &QString::from(out_dir),
            &QString::from(fixture),
            pixel_checks,
        )
    } else {
        1
    };
    drop(engine);
    drop(app);
    code
}

/// A deterministic tree big enough to lay several treemap tiles at
/// 1024x768: a handful of files with clearly distinct sizes, two levels of
/// directories, one empty directory, and one zero-byte file.
fn build_fixture() -> String {
    let root = format!(
        "{}/topograph-selftest-fixture-{}",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let _ = std::fs::remove_dir_all(&root);
    let blob = |bytes: usize| vec![b'x'; bytes];
    let files: &[(&str, usize)] = &[
        ("huge.bin", 4 * 1024 * 1024),
        ("medium.bin", 1500 * 1024),
        ("small.txt", 42 * 1024),
        ("tiny.txt", 512),
        ("zero.txt", 0),
    ];
    let dirs: &[&[&str]] = &[&["docs"], &["media"], &["docs", "nested"], &["empty"]];

    for dir in dirs {
        std::fs::create_dir_all(format!("{}/{}", root, dir.join("/"))).expect("mkdir fixture");
    }
    for (name, size) in files {
        std::fs::write(format!("{root}/{name}"), blob(*size)).expect("write fixture");
    }
    std::fs::write(format!("{root}/docs/notes.md"), blob(90 * 1024)).expect("write fixture");
    std::fs::write(format!("{root}/docs/nested/deep.rs"), blob(7 * 1024)).expect("write fixture");
    std::fs::write(format!("{root}/media/video.mkv"), blob(1024 * 1024)).expect("write fixture");
    std::fs::write(format!("{root}/media/audio.flac"), blob(600 * 1024)).expect("write fixture");
    root
}
