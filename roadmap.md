# Topograph Roadmap

The 20-phase master plan synthesized from `qdirstat`, `filelight`, and `baobab`. Each phase defines strict, granular execution targets.

- [x] Phase 0: **Project Skeleton & Qt Bindings**
  - [x] Initialize `topograph` and `topograph-core` Cargo workspaces.
  - [x] Scaffold portfolio documentation (README, spec, roadmap, patchnotes).
  - [x] Configure `cxx-qt` build dependencies and `build.rs` bridging logic.
  - [x] Define `qml/main.qml` as the application entry point.
  - [x] Map Kanagawa Dragon color variables into a global QML theme object.
  - [x] Implement initial CXX-Qt bridge demonstrating Rust -> QML passing.
  - [x] Configure `cargo fmt`, `clippy`, and strict compilation flags.
  - [x] Add `.gitignore` rules for CXX-Qt auto-generated C++ files.
  - [x] Create a dummy `Hello World` Qt6 Application Window.
  - [x] Verify clean build on local Linux environment (Wayland/X11 compatibility).
  *(All ten ticked 2026-09-04: every item is verifiably in the tree — build.rs,
  main.qml, the Kanagawa palette in QML, the working bridge, fmt/clippy now
  wired into CI, and target/CXX-Qt outputs gitignored. This Phase 0 copy had
  been left unticked while the same work was ticked under Phase 1a; the
  duplicates there are now the single source of truth.)*

- [x] Phase 1a: **Memory Architecture (Cache-friendly Arena)**
  - [x] Select and integrate an arena library (e.g., `indextree` or contiguous `Vec<Node>`).
  - [x] Configure `cxx-qt` build dependencies and `build.rs` bridging logic.
  - [x] Define `qml/main.qml` as the application entry point.
  - [x] Map Kanagawa Dragon color variables into a global QML theme object.
  - [x] Implement initial CXX-Qt bridge demonstrating Rust -> QML passing.
  - [x] Configure `cargo fmt`, `clippy`, and strict compilation flags.
  - [x] Add `.gitignore` rules for CXX-Qt auto-generated C++ files.
  - [x] Create a dummy `Hello World` Qt6 Application Window.
  - [x] Verify clean build on local Linux environment (Wayland/X11 compatibility).

- [x] Phase 1b: **Memory Architecture (Arena & Nodes)**
  - [x] Implement the `indextree` arena structure in `topograph-core`.
  - [x] Define the `NodeData` struct and `bitflags` for file type metadata.
  - [x] Implement the recursive post-order traversal for size aggregation.
  - [x] Validate $O(N)$ math performance on a 1,000,000 node synthetic test.
  - [x] Establish concurrent mutation safety patterns for the arena during building.
  - [x] Document the memory layout and invariants in `spec.md`.

- [x] Phase 2: **Concurrent Scanning Engine (Parallel Traversal)**
  - [x] Integrate `jwalk` or `rayon` for concurrent directory walking.
  - [ ] Implement POSIX-specific traversal using `rustix` `openat` and `fstatat`. *(Unticked 2026-09-04: the audit found no rustix/openat code — traversal is jwalk and metadata reads are std. Either implement or retire; it was falsely ticked.)*
  - [x] Force `AT_SYMLINK_NOFOLLOW` on all stat calls to prevent symlink loops.
  - [x] Implement an `AtomicBool` cancellation token for aborting active scans.
  - [x] Read `d_type` directly from directory entries to avoid redundant `stat` calls for directories.
  - [ ] Sort directories by inode number before traversing to minimize disk head seeks (rotational drive optimization). *(Unticked 2026-09-04: no inode-sorting code exists in the scanner.)*
  - [x] Handle `EACCES` (Permission Denied) gracefully, flagging nodes with an error state instead of crashing.
  - [x] Tune thread pool size to physical CPU cores to maximize IOPS without thread contention.
  - [x] Implement the bridging logic to stream scanned chunks back to the arena.
  - [x] Write a headless test harness running the scanner against a large system directory.

- [x] Phase 3: **Deduplication & File System Boundaries**
  - [x] Parse `/proc/mounts` at startup to build a list of external mounts (Solved via `st_dev` boundary checking).
  - [x] Compare `st_dev` (device ID) of directories against the root to prevent traversing into different filesystems.
  - [x] Implement a fast-path gate checking `st_nlink > 1` before performing hardlink deduplication.
  - [x] Create a sharded `DashMap` or partitioned `parking_lot::RwLock<HashSet>` for `(dev, inode)` tracking.
  - [x] Ensure the first encountered hardlink adds to total size; subsequent encounters add to file count but 0 to size.
  - [x] Add an opt-in toggle to allow crossing filesystem boundaries if explicitly requested.
  - [x] Add explicit checks to prevent traversing virtual file systems (Solved via `st_dev` checking).
  - [x] Test hardlink dedup against a synthetic test directory with multiple complex links.
  - [x] Write integration test verifying mount boundaries are strictly respected.
  - [x] Surface deduplicated savings (bytes saved) in the final UI metrics.

- [x] Phase 4: **Atomic UI Integration (Lock-free Progress)**
  - [x] Implement `AtomicU64` counters for `total_bytes` and `AtomicUsize` for `total_files`.
  - [x] Implement an `AtomicBool` state tracker to identify scan completion without Qt Threading traits.
  - [x] Expose these atomic variables to CXX-Qt via a read-only Rust bridge method.
  - [x] Create a 60FPS QML `Timer` that polls the Rust bridge and updates UI text.
  - [x] Calculate and display scan speed (e.g., "12,000 files/sec") via moving average.
  - [x] Ensure zero `Q_EMIT` signals are fired from worker threads to the UI to prevent event queue flooding.
  - [x] Build the minimal top-bar UI: "Scan Directory" button, path label, and progress text.
  - [x] Display an animated Kanagawa-styled indeterminate progress bar during scanning.
  - [x] Handle the "Scan Complete" signal transition to swap UI to the results view.
  - [x] Add a "Cancel" button that successfully halts the engine and resets the UI state.

- [x] **Phase 5 (ListModel hookup):** Hook the tree into QML. Hook `FileTree` into QML as a standard `QAbstractListModel` so that `TreeView` / `ListView` can inspect the hierarchy.tory tree.
  - [x] Map Qt roles (NameRole, SizeRole, PercentRole, IconRole) to Rust arena lookups.
  - [x] Implement lazy loading/expansion in the model to avoid instantiating millions of UI rows.
  - [x] Build the `TreeView` or `TableView` in QML with custom delegates for Kanagawa styling.
  - [x] Add formatting logic for human-readable sizes (B, KB, MB, GB, TB).
  - [x] Implement a small inline visual percentage bar (QML `Rectangle`) in the size column.
  - [ ] Bind keyboard navigation (Up/Down/Left/Right) to expand/collapse folders.
  - [ ] Ensure scrolling performance remains at 60FPS even with 100,000 expanded nodes.
  - [ ] Handle model invalidation/reset when a new scan completes.
  - [ ] Add sorting by Size (default), Name, or File Count.

- [ ] Phase 6: **Aggregation Math (Size & Percentages)**
  - [ ] Implement a post-order traversal over the arena to sum sizes from leaves to the root.
  - [ ] Calculate total allocated disk space vs apparent size.
  - [ ] Calculate maximum depth (`max_depth`) of the tree for rendering constraints.
  - [ ] Calculate `percentage = (child_size / parent_size) * 100.0` for every node.
  - [ ] Pre-calculate `rel_start` (cumulative percentage offset among siblings) for fast geometry.
  - [ ] Track total item counts (files + directories) per subtree.
  - [ ] Identify and flag the oldest and newest `mtime` in each subtree.
  - [ ] Store aggregated values cleanly back into the Arena nodes.
  - [ ] Ensure aggregation completes in < 50ms for a 1-million node tree.
  - [ ] Write regression tests verifying aggregation math against known hierarchical sizes.

- [ ] Phase 7: **Pseudo-nodes (`<Files>` and `<Ignored>`)**
  - [ ] Modify the aggregation pass to inject a `<Files>` pseudo-node under any directory containing both files and subdirectories.
  - [ ] Migrate all direct file children of that directory to be children of the `<Files>` node.
  - [ ] Inject an `<Ignored>` pseudo-node for paths skipped by exclusion filters.
  - [ ] Flag pseudo-nodes with a specific `NodeType` to render differently in the UI (italicized text).
  - [ ] Adjust percentage math so the `<Files>` node correctly represents the aggregate loose file size.
  - [ ] Add an option to toggle `<Files>` grouping on/off.
  - [ ] Ensure the treemap layout engine handles pseudo-nodes natively.
  - [ ] Write tests ensuring pseudo-nodes do not double-count total sizes.
  - [ ] Handle edge cases where a directory contains *only* files (skip pseudo-node creation).
  - [ ] Update the QAbstractListModel to support expanding pseudo-nodes.

- [ ] Phase 8: **Squarified Treemap Layout (Fast Math Geometry)**
  - [ ] Implement Bruls' Squarified Treemap packing algorithm in pure Rust.
  - [ ] Define the output primitive: `TreemapRect { x, y, w, h, node_id, depth }`.
  - [ ] Add a visual culling threshold (e.g., skip processing nodes whose calculated area is < 3x3 pixels).
  - [ ] Sort children by size descending before passing them to the row-packing logic.
  - [ ] Maintain an aspect ratio as close to 1.0 (square) as possible when slicing rectangles.
  - [ ] Return a flat `Vec<TreemapRect>` buffer from the layout engine, ready for GPU rendering.
  - [ ] Add cushion parameters to the math: calculate parabolic ridge coefficients based on depth.
  - [ ] Allow dynamic padding between directory rectangles to visualize hierarchy.
  - [ ] Benchmark layout generation: guarantee layout calculation for 1M files takes < 16ms.
  - [ ] Write synthetic layout tests to ensure aspect ratios remain mathematically bounded.

- [ ] Phase 9: **Cushion Treemap Rendering (GPU Fragment Shader)**
  - [ ] Create a custom `QQuickItem` / `QSGGeometryNode` in C++ to handle raw rendering.
  - [ ] Write the QML ShaderEffect / Fragment Shader for the cushion lighting equation.
  - [ ] Pass the flat `Vec<TreemapRect>` (including parabolic coefficients and color) to the GPU.
  - [ ] Calculate the dot product of the surface normal against a fixed light vector in the shader.
  - [ ] Apply ambient lighting and clamp the diffuse reflection.
  - [ ] Handle dynamic resizing: trigger a Rust layout recalculation and push the new buffer to the GPU.
  - [ ] Ensure 60FPS resizing performance without blocking the main Qt event loop.
  - [ ] Add antialiasing or 1px border lines to enforce contrast between adjacent tiles.
  - [ ] Expose lighting parameters (ambient intensity, light angle, cushion height) to the UI.
  - [ ] Verify GPU memory footprint remains negligible compared to instantiating QML elements.

- [ ] Phase 10: **Sunburst Layout Math (Radial Arc Geometry)**
  - [ ] Implement Filelight's 1/16th degree integer angular arithmetic in Rust.
  - [ ] Define the output primitive: `RadialArc { start_angle, span_angle, inner_radius, outer_radius, node_id }`.
  - [ ] Establish ring depth limits (e.g., max 5 levels deep) based on widget size.
  - [ ] Cull narrow arcs whose `span_angle` falls below a visual threshold.
  - [ ] Aggregate culled arcs into a "miscellaneous small files" pseudo-arc at the end of the ring.
  - [ ] Add the continuation arc logic (Baobab's outer line) for directories that have unrendered depth.
  - [ ] Return a flat `Vec<RadialArc>` buffer to the renderer.
  - [ ] Handle the center circle (root node) rendering logic.
  - [ ] Benchmark radial layout generation to ensure < 16ms execution.
  - [ ] Allow dynamic toggling between Treemap and Sunburst modes, swapping the math backend.

- [ ] Phase 11: **Sunburst Rendering (`QSGGeometryNode`)**
  - [ ] Implement a second custom `QSGGeometryNode` for drawing pie wedges/arcs.
  - [ ] Generate triangle fans or strips for each `RadialArc` directly in C++/Rust to avoid QML `ShapePath` overhead.
  - [ ] Pass base colors based on angle or Kanagawa palette maps.
  - [ ] Apply depth-based darkening (value/saturation shifts) per ring layer.
  - [ ] Implement the outer boundary continuation lines using thin stroke geometry.
  - [ ] Render the center root circle and its size text as an overlay.
  - [ ] Handle smooth window resizing by triggering Rust layout recalculation.
  - [ ] Ensure Z-ordering is correct (inner rings drawn on top of outer rings).
  - [ ] Optimize vertex counts (smooth arcs require sufficient segments based on radius).
  - [ ] Validate rendering correctness against empty directories and heavily skewed size distributions.

- [ ] Phase 12: **O(1) Interaction (Math-based Hit Testing)**
  - [ ] Implement an overarching `MouseArea` in QML that tracks `mouseX` and `mouseY`.
  - [ ] For Treemaps: Pass `(x, y)` to Rust. Perform a binary/quadtree search on sorted rects to find the hovered `NodeId`.
  - [ ] For Sunbursts: Convert `(x, y)` to polar coordinates and perform depth/angle binary search.
  - [ ] Avoid iterating through UI components entirely.
  - [ ] Emit a hovered `NodeId` signal back to QML.
  - [ ] Highlight the corresponding row in the `TreeList` side pane.
  - [ ] Highlight the hovered primitive in the GPU shader (pass a `hovered_node_id` uniform).
  - [ ] Render a tooltip at the cursor with the node's name, size, percentage, and file count.
  - [ ] Implement a debounced hover delay (100ms) to prevent tooltip flickering during fast movement.
  - [ ] Handle edge cases where the mouse is outside any rendered geometry.

- [ ] Phase 13: **Color Mapping (Kanagawa Dragon Semantics)**
  - [ ] Define the Kanagawa Dragon palette constants in Rust.
  - [ ] Implement a file-extension-to-category mapping (e.g., `.mp4` -> Video, `.rs` -> Code).
  - [ ] Assign specific Kanagawa anchor colors to file categories.
  - [ ] Assign folder colors based on a hash of their name, or their angular position mapped to Kanagawa hues.
  - [ ] Pass the computed RGB values down to the geometry primitives (`TreemapRect`, `RadialArc`).
  - [ ] Ensure contrast between adjacent nodes is preserved even when they share a category.
  - [ ] Implement a "Color by Depth" alternative mode.
  - [ ] Provide UI toggles to switch between color mapping strategies.
  - [ ] Store color preferences in application settings.
  - [ ] Add a legend in the UI indicating what file types map to what colors.

- [ ] Phase 14: **Drill-down Navigation (Changing Visualization Roots)**
  - [ ] Implement double-click on a geometry primitive or tree row to "zoom in".
  - [ ] Set the selected `NodeId` as the new visualization root.
  - [ ] Recalculate layout constraints (Squarified/Radial) treating the new root as 100% size.
  - [ ] Animate the transition if possible, or snap cleanly to the new geometry.
  - [ ] Add a breadcrumb navigation bar at the top of the UI (e.g., `Home > var > log > journal`).
  - [ ] Clicking a breadcrumb sets that ancestor `NodeId` as the root.
  - [ ] Add an "Up One Level" button and wire it to backspace/mouse-back-button.
  - [ ] Ensure the side pane `TreeList` auto-expands and scrolls to the selected node.
  - [ ] Update the center-circle text in the Sunburst chart to reflect the new root.
  - [ ] Test drill-down behavior on extremely deep directory structures.

- [ ] Phase 15: **Subtree Caching (Instantaneous Rescans)**
  - [ ] Implement an in-memory cache architecture.
  - [ ] Scan request on a known subtree bypasses disk entirely and renders instantly (`Cache-hit a`).
  - [ ] Scan request on a parent of an known tree reuses known branches and only scans missing paths (`Cache-hit b`).
  - [ ] Add a "Refresh" action that invalidates a specific `NodeId` and its children for a targeted disk rescan.
  - [ ] Seamlessly merge the targeted rescan results back into the global arena.
  - [ ] Recalculate aggregate sizes and geometry only for the affected branches.
  - [ ] Handle the case where the root directory was deleted or moved.
  - [ ] Provide a "Clear Cache" button in settings to dump the entire arena.
  - [ ] Add a timestamp to cached nodes to optionally auto-invalidate data older than X minutes.
  - [ ] Document the cache lifecycle state machine.

- [ ] Phase 16: **Action Interactivity (Context Menus)**
  - [ ] Implement a QML `Menu` triggered by right-clicking the visualization or tree list.
  - [ ] Add "Open in File Manager" (uses `xdg-open` or `dbus`).
  - [ ] Add "Open Terminal Here" (spawns default terminal emulator).
  - [ ] Add "Copy Path to Clipboard".
  - [ ] Add "Move to Trash" using the desktop trash specification.
  - [ ] Implement a safety confirmation dialog for Trash operations on items > 1GB.
  - [ ] Add "Delete Permanently" (with severe red warnings and confirmation).
  - [ ] Wire up file deletion to automatically trigger a cache invalidation and UI refresh for the parent node.
  - [ ] Implement multi-selection support in the TreeList for bulk operations.
  - [ ] Add a "Properties" dialog showing exact bytes, links, permissions, and dates.

- [ ] Phase 17: **Filter Pipeline (Exclusions & Dynamic Thresholding)**
  - [ ] Implement a global exclusion list in Rust (e.g., ignore `.git`, `node_modules`).
  - [ ] Apply exclusions during the concurrent scanning phase to prevent I/O entirely.
  - [ ] Add dynamic UI sliders for "Minimum visible rectangle size" (pruning threshold).
  - [ ] Add regex-based search functionality that filters the tree list and grays out non-matching visualization nodes.
  - [ ] Implement a "Hide files smaller than X MB" toggle.
  - [ ] Aggregate all hidden/filtered items into the `<Ignored>` pseudo-node dynamically.
  - [ ] Save the exclusion list to disk (e.g., `~/.config/topograph/excludes.toml`).
  - [ ] Add an interface in settings to add/remove exclusion patterns.
  - [ ] Test filter performance on large trees to ensure zero layout lag.
  - [ ] Implement "Only show videos/archives" quick-filter chips.

- [ ] Phase 18: **State Persistence (QSettings & Debouncing)**
  - [ ] Wire up `QSettings` or Rust `serde`+`toml` config file for persistent preferences.
  - [ ] Save and restore main window geometry (size, position, maximized state).
  - [ ] Save and restore pane splitter positions.
  - [ ] Save user preferences: default visualization mode, color scheme, padding.
  - [ ] Implement a debounce mechanism (e.g., 150ms timer) for window resize events.
  - [ ] Store the last scanned directory and optionally auto-scan it on next launch.
  - [ ] Ensure settings writes are non-blocking and atomic to prevent corruption.
  - [ ] Provide a "Reset to Defaults" button.
  - [ ] Add a localized string catalog mechanism (gettext or Qt tr).
  - [ ] Verify clean startup when the configuration file is missing or corrupted.

- [ ] Phase 19: **Optimization & Profiling**
  - [ ] Compile with `lto = "fat"`, `codegen-units = 1`, and `opt-level = 3`.
  - [ ] Run `valgrind` or `heaptrack` on CXX-Qt boundaries to ensure zero memory leaks during tree drops.
  - [ ] Profile with `perf` / `flamegraph` on a 2-million file filesystem (e.g., the root `/` drive).
  - [ ] Identify and eliminate any remaining lock contention in the scanning threads.
  - [ ] Optimize the arena's memory footprint (pack bitflags, reduce struct padding).
  - [ ] Benchmark startup time: target < 100ms from launch to UI ready.
  - [ ] Test on a severely resource-constrained VM (e.g., 2 cores, 2GB RAM).
  - [ ] Compare scan time and memory usage directly against QDirStat and Baobab.
  - [ ] Tune the cushion fragment shader for low-end integrated GPUs.
  - [ ] Document final performance metrics in `patchnotes.md`.

- [ ] Phase 20: **1.0 Release**
  - [ ] Finalize the `logo.svg` design and generate `.png`/`.ico` assets.
  - [ ] Write the user-facing `README.md` with installation and build instructions.
  - [ ] Create a `io.github.virinvictus.topograph.desktop` file.
  - [ ] Create the `io.github.virinvictus.topograph.metainfo.xml` (AppStream) for Linux app stores.
  - [ ] Draft a Flatpak manifest (`io.github.virinvictus.topograph.yml`) pulling in Rust and Qt6 KDE runtimes.
  - [ ] Verify offline build capability for the Flatpak (vendored Cargo sources).
  - [ ] Take high-resolution Kanagawa Dragon themed screenshots for the portfolio and metainfo.
  - [ ] Complete a final manual QA pass of all interactive features.
  - [ ] Cut the `v1.0.0` git tag.
  - [ ] Write the release announcement in `patchnotes.md`.

- [ ] Post-1.0 Phase: **TUI Mode (Terminal User Interface)**
  - [ ] Implement a new `topograph-tui` crate in the workspace dependent on `topograph-core`.
  - [ ] Reference existing portfolio TUI idioms and layouts from `lattice-music` and `../CalibreQuarry` to ensure cross-project UX consistency.
  - [ ] Select a Rust TUI framework (e.g., `ratatui`).
  - [ ] Implement a dual-pane terminal layout matching the GUI (Tree on left, visualization on right).
  - [ ] Build a text-based Squarified Treemap renderer using block drawing characters (Braille or half-blocks).
  - [ ] Map the Kanagawa Dragon palette to ANSI escape codes for the terminal.
  - [ ] Wire up keyboard navigation (Vim bindings: `h`, `j`, `k`, `l`) for tree traversal.
  - [ ] Bind atomic progress counters to a terminal progress bar during the scan phase.
  - [ ] Ensure graceful fallback if the terminal does not support truecolor.
  - [ ] Add CLI arguments (`--tui`) to launch directly into the terminal mode instead of Qt.
  - [ ] Write documentation for the TUI mode in `README.md`.
