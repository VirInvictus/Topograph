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
//! cannot draw the treemap's raw QSGGeometryNode. If the GPU path is dead
//! OR renders as software (Qt can fall back silently; the capture walk
//! detects it), the process re-execs itself once on the software backend,
//! where the treemap's pixel checks print SKIP lines instead of silently
//! passing. Overrides: TOPOGRAPH_SELFTEST_GL=1 (no fallback),
//! TOPOGRAPH_SELFTEST_SOFTWARE=1 (software directly), plus the usual
//! QT_QPA_PLATFORM/QT_QUICK_BACKEND.
//!
//! Set TOPOGRAPH_SELFTEST_RECORD=1 to append a ~15s animation phase after
//! the checks (light sweep, padding sweep, resize sweep; a frame per step
//! under <out>/record/) and encode it with ffmpeg when available.
//!
//! Output lands in TOPOGRAPH_SELFTEST_OUT (default /tmp/topograph-selftest)
//! rather than argv so the bridge's launch-argument seed (argv[1]) stays
//! empty and the window shows the scanned fixture path, not the output
//! directory. The exit code is zero only when every requested step passed.

mod bridge;
mod directory_model;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let force_software = std::env::var_os("TOPOGRAPH_SELFTEST_SOFTWARE").is_some();
    let no_fallback = std::env::var_os("TOPOGRAPH_SELFTEST_GL").is_some();

    // The display environment is stripped UNCONDITIONALLY, not merely
    // defaulted to offscreen: with WAYLAND_DISPLAY present, a platform
    // fallback or plugin failure surfaces the window on the user's real
    // desktop. This actually happened on 2026-10-07 — a selftest window
    // tiled itself into the live Hyprland session mid-run (the "resize did
    // not take" flakes were the compositor overriding tiled-window sizes).
    // Safety: single-threaded startup before any threads exist.
    unsafe {
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("DISPLAY");
        std::env::set_var("QT_QPA_PLATFORM", "offscreen");
        if force_software {
            std::env::set_var("QT_QUICK_BACKEND", "software");
        }
    }

    let out_dir = std::env::var("TOPOGRAPH_SELFTEST_OUT")
        .unwrap_or_else(|_| "/tmp/topograph-selftest".to_string());
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
            .env("TOPOGRAPH_SELFTEST_SOFTWARE", "1")
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("QT_QUICK_BACKEND", "software")
            .status()
            .expect("selftest re-exec");
        std::process::exit(status.code().unwrap_or(1));
    }

    if code == 0 {
        // The record phase ran inside the capture walk when requested;
        // encode its frames when ffmpeg is available.
        if std::env::var_os("TOPOGRAPH_SELFTEST_RECORD").is_some() {
            let frames = format!("{out_dir}/record");
            let video = format!("{out_dir}/selftest.mp4");
            let status = std::process::Command::new("ffmpeg")
                .args([
                    "-y",
                    "-framerate",
                    "4",
                    "-i",
                    &format!("{frames}/%03d.png"),
                    "-c:v",
                    "libx264",
                    "-pix_fmt",
                    "yuv420p",
                    "-crf",
                    "20",
                    &video,
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            match status {
                Ok(s) if s.success() => println!("recording encoded: {video}"),
                _ => println!("frames in {frames} (ffmpeg unavailable or failed)"),
            }
        }
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

/// A deterministic tree sized to exercise the layout the way a real
/// directory does: a clear largest file but no 50%-dominant slab, several
/// mid-size tiles, two levels of directories, one empty directory, and one
/// zero-byte file. The first fixture had a 58%-of-total file and rendered
/// as one flat slab, which read as a rendering bug when it was only the
/// fixture.
fn build_fixture() -> String {
    let root = format!(
        "{}/topograph-selftest-fixture-{}",
        std::env::temp_dir().display(),
        std::process::id()
    );
    let _ = std::fs::remove_dir_all(&root);
    let blob = |kb: usize| vec![b'x'; kb * 1024];

    for dir in ["docs", "media", "docs/nested", "code", "empty"] {
        std::fs::create_dir_all(format!("{root}/{dir}")).expect("mkdir fixture");
    }
    let files: &[(&str, usize)] = &[
        ("biggest.bin", 900), // KB; ~22% of the total, a real largest tile
        ("media/video.mkv", 700),
        ("medium.bin", 600),
        ("media/audio.flac", 450),
        ("docs/notes.md", 400),
        ("docs/nested/deep.rs", 300),
        ("code/main.rs", 260),
        ("alpha.dat", 210),
        ("beta.dat", 150),
        ("code/lib.rs", 120),
        ("gamma.dat", 90),
        ("small.txt", 42),
        ("tiny.txt", 4),
        ("zero.txt", 0),
    ];
    for (name, kb) in files {
        std::fs::write(format!("{root}/{name}"), blob(*kb)).expect("write fixture");
    }
    root
}
