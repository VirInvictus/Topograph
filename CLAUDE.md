# CLAUDE.md

## Topograph

A fast, local-first file system size explorer with a squarified cushion treemap view.

**Language:** Rust 2024
**Framework:** Qt6 / QML (via CXX-Qt)

- Build: `cargo build`
- Run: `cargo run -p topograph`
- Tests: `cargo test`

Note: This project relies on Kanagawa Dragon for its styling. Two views ship: the plain QML ListView over the Rust core (tree mode) and the treemap (a custom C++ `QQuickItem` rendering one `QSGGeometryNode`). The sunburst phases and the rest of Phases 10-20 + TUI remain aspirational scope, marked as such at roadmap.md:4. No libadwaita or GTK logic exists here anymore.

## Model notes

- `DirectoryModel` is a flat `Vec<NodeDisplay>` under one root index; rows carry
  arena `NodeId`s. `expandRow`/`collapseRow` splice in or remove a row's direct
  children and publish with a full model reset, so only expanded levels exist
  as rows. A collapsed row owns every following row deeper than itself.
- Sorting lives on the model: `sortBy(key, descending)` (QML header) reorders
  each sibling run with `sort_range`, whole subtrees moving with their parents,
  inside the begin/endResetModel pair. New children from `expandRow` inherit
  the active sort. Keys: size (default, descending), name (natural order:
  case-insensitive, numeric digit runs), count (per-subtree item count).
- The Percent role is the row's aggregate size over its parent's aggregate
  size, stored on every node by the aggregation's sibling-offset pass since
  v0.4.0 (`NodeData.percent`; the model reads the stored share at row-build
  and hardcodes the root row at 100); the QML delegate draws it as an inline
  bar in the size column. The count
  column reads FileCount (real since v0.3.2), and sizes render through the
  `formatSize` invokable (binary-unit B/KB/MB/GB/TB in
  `bridge::format_size`, shared with the progress line).
- `LATEST_TREE` (bridge.rs) is the shared scan-result slot. `loadTree` rebuilds
  from it when the bridge emits `scanFinished` (from `update_metrics` once the
  worker reports completion). The old mechanism keyed on the UI string
  `progressText === "Scan complete."` and never fired: `is_scanning` flipped
  before the text was written, so the handler always read the stale value.
  Publication is generation-guarded (`TREE_GENERATION` in bridge.rs): starting
  or cancelling a scan invalidates the in-flight worker's late publish. A
  failed scan (root missing or unreadable: `ScanMetrics.failed`) publishes
  nothing and skips the `scanFinished` emit, so the model keeps the tree it
  had; the progress line is the error channel ("Scan failed: cannot read
  <path>"). Completion reports both totals ("N files, X apparent / Y on disk
  in Zs", plus "; dedup saved W" only when hardlinks deduplicated anything)
  and the window
  title follows the scanned path. `topograph <path>` seeds the scan field
  through the `initial_path` property (read from std::env::args at bridge
  construction).
- The `force_link()` stubs (bridge.rs, directory_model.rs) are load-bearing
  CXX-Qt 0.6 anti-stripping shims. Never delete them.
- Keyboard navigation is QML-side (main.qml): the ListView holds focus and draws a
  highlight; Up/Down are its built-in navigation, Left/Right call
  `collapseRow`/`expandRow` on the current row. Every expand, collapse, and sort
  publishes a full model reset, which clears `currentIndex`, so each QML wrapper
  snapshots the current row's name+depth before the call and re-finds it after
  (an `itemAtIndex` scan outward from the old index over the instantiated
  delegates), falling back to the old index when the operation removed the row.
  At current call sites that fallback is unreachable: the click handler selects
  the clicked row before collapsing it, so the snapshot is of the collapsing
  row itself and always re-finds it ("collapse with a descendant selected ends
  on the collapsed parent" is that re-targeting, not the fallback), and
  keyboard Left only ever collapses the selected row, never an ancestor.
  Row clicks and scan completion hand focus to the tree so the arrow
  keys work immediately.
- The FileCount role carries the per-subtree item count written by
  `aggregate_sizes` (files + directories, each directory counting itself);
  leaves store 1. Before v0.3.2 it was hardcoded 0.
- The treemap (v0.4.0): all layout, cushion, and lighting math lives in
  `topograph-core/src/treemap.rs` (squarified packing, culling below a
  3-pixel square, depth-tinted Kanagawa directory colors, per-vertex cushion
  shading on an area-bounded grid). The GUI side is a hand-written C++
  `TreemapView` (`topograph/src/treemap_view.{h,cpp}`, mocz'd via
  `qobject_header` in build.rs, compiled by the same cc build as the
  CXX-Qt-generated sources, registered as a QML type under
  `com.topograph.treemap` from a static initializer). The data path is two
  extern-"Rust" functions, `treemap_rebuild`/`treemap_copy_vertices`: the
  buffer is built into the `TREEMAP_VERTICES` slot (same global-slot pattern
  as `LATEST_TREE`) as packed x/y-f32 + rgba8 bytes (Qt's colored-point
  layout, 12 bytes/vertex) and copied into the QSGGeometry in one call, so
  there is no per-vertex FFI. `bridge::force_link` calls
  `topograph_treemap_force_link()` (defined in treemap_view.cpp) to anchor
  that object file against linker stripping; treat it like the other
  force_link stubs.
- Why the cushion lighting is per-vertex, not a fragment shader: a custom
  `QSGMaterialShader` requires `.qsb`-baked shaders (qtshadertools), which
  the CI Qt 6.6.0 default install does not carry (checked against Qt's
  repository metadata, 2026-10-07). The equation is identical, evaluated at
  grid vertices in Rust and interpolated by hardware. If CI ever grows
  qtshadertools, the fragment-shader box on roadmap Phase 9 is the
  pull-forward.
- GUI verification is headless: `cargo run -p topograph --bin
  topograph-selftest` loads the real main.qml on Qt's offscreen platform,
  starts a scan by invoking the bridge directly (no synthetic input,
  no compositor), flips to the treemap, toggles the cushion, resizes to
  1400x900, and captures a frame after every step via
  QQuickWindow::grabWindow (the walk lives in src/selftest_support.cpp;
  the fixture and graphics negotiation in src/selftest.rs). Exit 0 means
  every step passed; captures land in the output dir (default
  /tmp/topograph-selftest). Graphics: the default run tries the GPU path
  (offscreen RHI; works here via Mesa surfaceless EGL + llvmpipe) because
  the software scene graph cannot draw the treemap's raw QSGGeometryNode;
  if the GPU path is dead it re-execs once on software, where the
  treemap pixel checks print SKIP lines. Env overrides:
  TOPOGRAPH_SELFTEST_GL=1 (no fallback), TOPOGRAPH_SELFTEST_SOFTWARE=1.
  Dead ends recorded (do not retry): Qt's VNC platform segfaults under Qt
  Quick, and this Fedora weston ships no screenshooter module plus a
  TLS/RSA-AES-only VNC backend, so compositor-based headless capture is a
  dead end on this machine.
