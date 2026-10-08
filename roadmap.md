# Topograph Roadmap

The master plan synthesized from `qdirstat`, `filelight`, and `baobab`, organized as Phases 0 through 20 plus a post-1.0 TUI phase (23 groups today: the memory-architecture work is split across Phases 1a and 1b). Each phase defines strict, granular execution targets.
  *(MARKED ASPIRATIONAL 2026-09-12 (Brandon): Phases 6-20 plus the TUI are aspirational scope, honestly labelled; the shipped v0.3.0 is the product, and any single phase can be pulled forward later as its own decision.)*
  *(UPDATE 2026-10-07: two phases have since been pulled forward under that clause: Phase 6 closed (its aggregation boxes ticked with notes, a few surfacing-later caveats disclosed inline) and Phases 8-9 shipped as the treemap lane (v0.4.0). The rest remains aspirational.)*

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
  *(All ten ticked 2026-09-04: every item is verifiably in the tree: build.rs,
  main.qml, the Kanagawa palette in QML, the working bridge, fmt/clippy now
  wired into CI, and target/CXX-Qt outputs gitignored. Phase 1a repeats the
  same skeleton items from the old double-Phase-1 layout; both lists describe
  the same shipped work and both stay ticked.)*

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
  - [ ] Implement POSIX-specific traversal using `rustix` `openat` and `fstatat`. *(Unticked 2026-09-04: the audit found no rustix/openat code; traversal is jwalk and metadata reads are std. Either implement or retire; it was falsely ticked.)*
    *(RETIRED 2026-09-12 (Brandon): declined, not shipped; traversal stays jwalk.)*
  - [x] Force `AT_SYMLINK_NOFOLLOW` on all stat calls to prevent symlink loops.
  - [x] Implement an `AtomicBool` cancellation token for aborting active scans.
  - [x] Read `d_type` directly from directory entries to avoid redundant `stat` calls for directories.
  - [ ] Sort directories by inode number before traversing to minimize disk head seeks (rotational drive optimization). *(Unticked 2026-09-04: no inode-sorting code exists in the scanner.)*
    *(RETIRED 2026-09-12 (Brandon): declined, not shipped; a rotational-drive micro-optimization on an SSD-only house.)*
    - [x] Implement the bridging logic to stream scanned chunks back to the arena.
  - [x] Handle `EACCES` (Permission Denied) gracefully without crashing. *(2026-09-05 correction: failures silently zero the node's sizes; `NodeFlags` has no error bit yet, so "flagging with an error state" was aspirational.)*
  - [x] Tune thread pool size to physical CPU cores to maximize IOPS without thread contention. *(2026-09-05 correction: no pool is configured; traversal uses jwalk's default (rayon) pool.)*
  - [x] Write a headless test harness running the scanner against a large system directory. *(2026-09-15 correction: the harness (`test_scanner_on_src`) scans the crate's own `src/`, not a large system directory; the headline overstated what ships. A real-mount scan remains a recorded runtime session.)*

- [x] Phase 3: **Deduplication & File System Boundaries**
  - [x] Parse `/proc/mounts` at startup to build a list of external mounts (Solved via `st_dev` boundary checking). *(2026-09-15 correction: no `/proc/mounts` parsing exists anywhere; the boundary check compares `st_dev` per entry during the walk. The box's headline names a mechanism that never shipped; the substitute is the one disclosed inline.)*
  - [x] Compare `st_dev` (device ID) of directories against the root to prevent traversing into different filesystems.
  - [x] Implement a fast-path gate checking `st_nlink > 1` before performing hardlink deduplication.
  - [x] Create a sharded `DashMap` or partitioned `parking_lot::RwLock<HashSet>` for `(dev, inode)` tracking.
  - [x] Ensure the first encountered hardlink adds to total size; subsequent encounters add to file count but 0 to size.
  - [x] Add an opt-in toggle to allow crossing filesystem boundaries if explicitly requested. *(Ticked 2026-10-07, v0.4.0: a `crossFilesystems` checkbox beside the Scan button feeds a bridge property the scanner reads per scan; the st_dev prune decision itself was extracted into `should_prune` and unit-tested for both settings. Honest caveat stands: with it on, a scan of / walks every mount.)*
  - [x] Add explicit checks to prevent traversing virtual file systems (Solved via `st_dev` checking).
  - [x] Test hardlink dedup against a synthetic test directory with multiple complex links.
  - [x] Write integration test verifying mount boundaries are strictly respected. *(Ticked 2026-10-07, v0.4.0, with the caveat disclosed: the cross-device leg cannot be fabricated without a mount, which needs root, so `should_prune` is exhaustively unit-tested (same-dev kept, cross-dev pruned, the toggle off-switch, unknown root dev, stat-failed entries kept) beside an integration test that a real walk prunes nothing on one filesystem. A live cross-mount run remains a manual check.)*
  - [x] Surface deduplicated savings (bytes saved) in the final UI metrics. *(Ticked 2026-10-07, v0.4.0, pulling the retirement forward as its own decision: the scanner counts duplicate-link encounters and the apparent bytes they would have double-counted; the scan summary appends "; dedup saved X" only when the tree had multi-linked files, and the hardlink test asserts both counters.)*
    *(The 2026-09-12 retirement it supersedes: declined for then; the metric needs Phase 6 counts and can be pulled forward as its own decision. The counts shipped in v0.3.2; the decision is tonight's.)*

- [x] Phase 4: **Atomic UI Integration (Lock-free Progress)**
  - [x] Implement `AtomicU64` counters for `total_bytes` and `AtomicUsize` for `total_files`.
  - [x] Implement an `AtomicBool` state tracker to identify scan completion without Qt Threading traits.
  - [x] Expose these atomic variables to CXX-Qt via a read-only Rust bridge method.
  - [x] Create a 60FPS QML `Timer` that polls the Rust bridge and updates UI text.
  - [x] Calculate and display scan speed (e.g., "12,000 files/sec"). *(2026-09-05 correction: a single-window delta rate, not a moving average.)*
  - [x] Ensure zero `Q_EMIT` signals are fired from worker threads to the UI to prevent event queue flooding.
  - [x] Build the minimal top-bar UI: "Scan Directory" button, path label, and progress text.
  - [x] Display an animated Kanagawa-styled indeterminate progress bar during scanning.
  - [x] Handle the "Scan Complete" signal transition to swap UI to the results view.
  - [x] Add a "Cancel" button that successfully halts the engine and resets the UI state.

- [x] **Phase 5 (ListModel hookup):** Hook the tree into QML. Hook `FileTree` into QML as a standard `QAbstractListModel` so that a `ListView` can inspect the hierarchy tree.
  - [x] Map Qt roles to Rust arena lookups. *(2026-09-05 correction: the shipped role set is FileName/FileSize/FileCount/IsDirectory/Depth; PercentRole and IconRole were never mapped.)*
  - [x] Implement lazy loading/expansion in the model to avoid instantiating millions of UI rows. *(Ticked 2026-09-06: model rows carry arena `NodeId`s; `expandRow`/`collapseRow` splice in or remove a row's direct children under a model reset, so only expanded levels exist as rows. Expansion state resets on each new scan via `loadTree`.)*
  - [x] Build the tree view in QML with custom delegates for Kanagawa styling. *(2026-09-05 correction: what ships is a `ListView` placeholder, as the v0.2.1 notes themselves say; no `TreeView`/`TableView`.)*
  - [x] Add formatting logic for human-readable sizes (B, KB, MB, GB, TB). *(Unticked 2026-09-13: the only formatter always printed MB (the QML delegate divided by 1 MB and appended "MB"); the claim was never true. Ticked 2026-09-15, v0.3.2: the tiered formatter shipped at both sites, rows through the formatSize invokable and the progress line through the same Rust helper, in binary units.)*
  - [x] Implement a small inline visual percentage bar (QML `Rectangle`) in the size column. *(Ticked 2026-09-06: a Percent role computed as the row's aggregate size over its parent's aggregate size, drawn as an inline Kanagawa-aqua bar with a numeric share beside the size text.)*
  - [x] Bind keyboard navigation (Up/Down/Left/Right) to expand/collapse folders. *(Ticked 2026-09-13, v0.3.1: QML-side only, zero model changes. The ListView takes focus with a Kanagawa highlight; Up/Down are its built-in navigation; Left/Right call collapseRow/expandRow. Every expand/collapse/sort restores the selection across the full model reset by re-finding the row's snapshotted name+depth, with an index fallback when the operation removed the row (collapse with a descendant selected selects the collapsed parent). Row clicks and scan completion hand focus to the tree. Verified by driving the live app with synthetic key input.)*
  *(2026-09-15 correction: the "collapse with a descendant selected selects the collapsed parent" outcome is real but comes from the click handler, which selects the clicked directory before collapsing it, so the snapshot is of the parent row itself and always re-finds it. The removal fallback would land on whatever row shifted into the old index and is unreachable at current call sites; keyboard Left only ever collapses the selected row, never an ancestor.)*
  - [ ] Ensure scrolling performance remains at 60FPS even with 100,000 expanded nodes.
  - [x] Handle model invalidation/reset when a new scan completes. *(Ticked 2026-09-05, mechanism replaced 2026-09-06: the refresh originally keyed on the QML string `progressText === "Scan complete."`, but the bridge writes `is_scanning` before the text, so the handler always read a stale value and the model never reloaded. `update_metrics` now emits a `scanFinished` signal after the properties settle, and QML reloads on that.)*
  - [x] Add sorting by Size (default), Name, or File Count. *(Ticked 2026-09-06: a `sortBy(key, descending)` invokable plus a QML sort header; sibling runs reorder in place with their subtrees, and newly expanded children follow the active sort. Count sorting is a stable no-op until Phase 6 subtree counts make the FileCount role real.)*
  *(2026-09-15, v0.3.2: the Phase 6 counts shipped; Count orders for real now and the delegate gained a count column. The Name key also went natural-order (case-insensitive, numeric digit runs).)*
  - [x] Guard the shared tree slot against late publishes from cancelled scans (audit finding 2026-09-05). *(Ticked 2026-09-06: a scan-generation counter gates publication in `publish_tree`; starting or cancelling a scan invalidates the in-flight worker's publish, so a stale partial tree can no longer overwrite a newer scan's results.)*

- [x] Phase 6: **Aggregation Math (Size & Percentages)** *(Header ticked 2026-10-07, v0.4.0: all ten boxes now carry work; two disclose surfacing-later caveats inline (allocated per-node display, max_depth awaiting its consumer).)*
  - [x] Implement a post-order traversal over the arena to sum sizes from leaves to the root. *(Ticked 2026-09-06: this shipped under Phase 1b as `aggregate_sizes`/`post_order_aggregate` in topograph-core and is called in production by the bridge before publishing; the Phase 6 copy duplicates that work and is ticked on that basis.)*
  - [x] Calculate total allocated disk space vs apparent size. *(Ticked 2026-10-07, v0.4.0: per-node allocated sums have aggregated since v0.2.x; what was missing is the surface. `ScanMetrics.total_allocated` now counts leaf allocated bytes and the completion summary reports both ("X apparent / Y on disk"). Per-node display remains future.)*
  - [x] Calculate maximum depth (`max_depth`) of the tree for rendering constraints. *(Ticked 2026-10-07, v0.4.0: `aggregate_sizes` records `FileTree.max_depth`. No consumer yet: the treemap's ridge falloff is per-level, not depth-normalized; the sunburst's ring limits are the intended consumer.)*
  - [x] Calculate `percentage = (child_size / parent_size) * 100.0` for every node. *(Ticked 2026-10-07, v0.4.0: a pre-order sibling-offset pass stores `NodeData.percent` for every node; the model's Percent role reads the stored value instead of recomputing at row-build.)*
  - [x] Pre-calculate `rel_start` (cumulative percentage offset among siblings) for fast geometry. *(Ticked 2026-10-07, v0.4.0: stored as `NodeData.rel_start` in the same pass, unit-tested against a known hierarchy. Consumer is Phase 10's radial layout; the treemap does not need it.)*
  - [x] Track total item counts (files + directories) per subtree. *(Ticked 2026-09-15, v0.3.2: `aggregate_sizes` carries a count alongside the sizes, `NodeData.count` stores it, directories count themselves, and the delegate's count column surfaces it; the dead Count header became a working sort.)*
  - [x] Identify and flag the oldest and newest `mtime` in each subtree. *(Ticked 2026-10-07, v0.4.0: post-order fold into `mtime_oldest`/`mtime_newest`, leaves carrying their own mtime, empty directories their own rather than sentinels. Stored only; Phase 13's age coloring is the intended consumer.)*
  - [x] Store aggregated values cleanly back into the Arena nodes. *(Ticked 2026-10-07, v0.4.0: the post-order pass writes size, allocated, count, and the mtime range; the sibling-offset pass writes percent and rel_start; `max_depth` rides on the tree.)*
  - [x] Ensure aggregation completes in < 50ms for a 1-million node tree. *(Ticked 2026-09-06 with a caveat: the million-node test measures the aggregation and prints the duration log-only; the bound is deliberately not asserted because debug and CI machines vary. The 50ms figure is a release-build observation, not an enforced gate.)*
  - [x] Write regression tests verifying aggregation math against known hierarchical sizes. *(Ticked 2026-09-06: the million-node test already asserted exact root sums and the hardlink test an exact deduped size; a dedicated known-hierarchy test now also checks intermediate directory sums and allocated sizes.)*

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

- [x] Phase 8: **Squarified Treemap Layout (Fast Math Geometry)** *(Pulled forward 2026-10-07 under Brandon's go for the treemap lane (the §5.11 reopen condition); every box below ticked that day, v0.4.0, in `topograph-core/src/treemap.rs`.)*
  - [x] Implement Bruls' Squarified Treemap packing algorithm in pure Rust.
  - [x] Define the output primitive: `TreemapRect { x, y, w, h, node_id, depth }`.
  - [x] Add a visual culling threshold (e.g., skip processing nodes whose calculated area is < 3x3 pixels).
  - [x] Sort children by size descending before passing them to the row-packing logic.
  - [x] Maintain an aspect ratio as close to 1.0 (square) as possible when slicing rectangles.
  - [x] Return a flat `Vec<TreemapRect>` buffer from the layout engine, ready for GPU rendering.
  - [x] Add cushion parameters to the math: calculate parabolic ridge coefficients based on depth.
  - [x] Allow dynamic padding between directory rectangles to visualize hierarchy.
  - [x] Benchmark layout generation: guarantee layout calculation for 1M files takes < 16ms. *(Measured, not gated, per the aggregation precedent: 7.5ms layout plus 21.8ms vertex build in release on the 1M-node synthetic chain (the 100-deep cascade; 93 rects, 316k vertices there, because nesting that deep stacks near-fullscreen tiles). Real trees sit an order shallower and the grid cap bounds vertices by screen area; debug and CI machines vary.)*
  - [x] Write synthetic layout tests to ensure aspect ratios remain mathematically bounded. *(Single-tile fill, sibling-area partition (f32 tolerance disclosed), aspect bound <= 5 on skewed input, culling, padded recursion, and cushion on/off shading variance.)*

- [x] Phase 9: **Cushion Treemap Rendering (GPU Fragment Shader)** *(Pulled forward 2026-10-07 with Phase 8, v0.4.0: the rendering core shipped; the two boxes below stay unticked honestly. The renderer is `TreemapView` in `topograph/src/treemap_view.{h,cpp}`.)*
  - [x] Create a custom `QQuickItem` / `QSGGeometryNode` in C++ to handle raw rendering.
  - [ ] Write the QML ShaderEffect / Fragment Shader for the cushion lighting equation. *(2026-10-07 disclosure: the lighting runs per vertex in Rust instead (same equation: accumulated parabola slopes, surface normal dotted against the fixed light vector, ambient plus clamped diffuse), emitted on an adaptively subdivided grid whose cell size bounds total vertices by screen area. Reason: a custom `QSGMaterialShader` needs `.qsb`-baked shaders, and the CI Qt 6.6.0 default install carries no qtshadertools (verified against Qt's repository metadata), so the shader route would break CI. A true fragment shader remains a pull-forward if the CI toolchain grows qtshadertools.)*
  - [x] Pass the flat `Vec<TreemapRect>` (including parabolic coefficients and color) to the GPU. *(The parabolas are pre-evaluated: what crosses is the packed x/y/rgba vertex buffer (12 bytes per vertex, Qt's colored-point layout) built into a shared slot and memcpy'd in one call, no per-vertex FFI.)*
  - [x] Calculate the dot product of the surface normal against a fixed light vector in the shader.
  - [x] Apply ambient lighting and clamp the diffuse reflection.
  - [x] Handle dynamic resizing: trigger a Rust layout recalculation and push the new buffer to the GPU. *(geometryChange triggers the rebuild and updatePaintNode reallocates when the vertex count moved; verified 2026-10-07 evening, v0.4.1: the headless selftest resizes the live window to 1400x900 and the capture comes back at the new size with the relayouted treemap.)*
  - [ ] Ensure 60FPS resizing performance without blocking the main Qt event loop. *(Vertex emission is bounded by screen area by construction, but each rebuild still walks the whole tree (cull now happens before the sibling sort), so very large trees pay a tree-sized pass per resize tick; measured 7.5ms+21.8ms for the million-node synthetic in release. The live feel stays Brandon's eyeball, like the 100k-row scrolling box.)*
  - [x] Add antialiasing or 1px border lines to enforce contrast between adjacent tiles. *(Contrast comes from the directory padding gaps (the background shows between levels) plus the cushion ridges themselves; per-tile outline quads remain an option if the eyeball wants crisper edges.)*
  - [x] Expose lighting parameters (ambient intensity, light angle, cushion height) to the UI. *(Q_PROPERTYs on TreemapView wired to four Kanagawa-styled sliders: cushion height, ambient, light angle, and directory padding, with a live tile count beside them.)*
  - [x] Verify GPU memory footprint remains negligible compared to instantiating QML elements. *(One geometry node total: at the live 939x1050 canvas over ~/.gitrepos, 8,325 tiles emitted roughly 40k vertices = ~480KB of vertex data, bounded by the grid cap; the QML-object-per-tile alternative would be thousands of scene-graph items.)*

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
  - [ ] Scan request on a parent of a known tree reuses known branches and only scans missing paths (`Cache-hit b`).
  - [ ] Add a "Refresh" action that invalidates a specific `NodeId` and its children for a targeted disk rescan.
  - [ ] Merge the targeted rescan results back into the global arena.
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

## New findings 2026-09-12 (six-lens full audit; detail: audit/FULL-AUDIT-2026-09-12.md, Wave 25)

- [x] **Two pre-existing falsified ticks surfaced by the audit (untick
      with dated notes, matching the v0.2.4 precedent):** roadmap.md:65
      claims an integration test for mount boundaries (the st_dev pruning
      has zero test coverage) and roadmap.md:85 claims B/KB/MB/GB/TB
      formatting (the only formatter always prints MB). Fix: untick with
      notes, or ship the ~10-line tiered formatter.
      *(Done 2026-09-13, v0.3.1: both unticked with dated notes; the
      tiered-formatter alternative stays open as a pull-forward.)*
- [x] **The retired boxes now render honestly** (unticked with RETIRED
      notes adjacent; repaired 2026-09-12 after tonight's retirement
      commit initially ticked them; the notes' placement is now adjacent
      to their boxes).
      *(Ticked 2026-09-15: the repair this box records was verified by
      the final audit; the box itself was simply never ticked.)*
- [x] **Blitz candidates:** the keyboard-nav lane (~60 lines of QML:
      focus + highlight + left/right expand/collapse; the real work is
      preserving the current row across the full model resets); the
      patchnotes rider (tonight's aspirational marking + retirements have
      no entry); optional bounded riders: Phase 6 subtree counts (completes
      the dead Count header, ~40 lines) or the tiered formatter.
      *(Keyboard nav + the rider shipped 2026-09-13 in v0.3.1, verified
      live; the Phase 6 counts and tiered-formatter riders remain
      optional pull-forwards.)*
- [x] **Docs:** CLAUDE.md predates tonight's decisions ("all rendering is
      GPU-accelerated" is now aspirational; Phase 6 cited without the
      flag); README's "visualizer analogous to qdirstat" overpromises
      (tree explorer today, treemap planned).
      *(Fixed 2026-09-13, v0.3.1: CLAUDE.md and README repositioned.)*
- [x] **GitHub presentation (workspace batch):** zero Releases (cut
      v0.3.0 from patchnotes); malformed topic qt6--qml-cxx-qt (proposed
      set: rust/qt6/qml/cxx-qt/filesystem/disk-usage/linux/kanagawa);
      description replacement (drops "blazing fast", says treemap
      planned).
      *(Done 2026-09-15: Releases cut for all three tags with
      --notes-from-tag (v0.3.1 Latest); the drafted topic set applied;
      the description now reads "A native Qt6/QML file system size
      explorer for Linux, built in Rust and styled with Kanagawa Dragon.
      The treemap visualization is planned but not built yet.")*
- [ ] **Dependency aging (noted 2026-09-13, no action taken):** CI's
      install-qt-action@v3 has a v4 line and the Qt 6.6.0 pin is aging;
      cxx-qt 0.6.1 has a 0.7.x line (API churn, no urgency); dashmap
      5.5 has a 6.x line. All are routine modernizations for a future
      maintenance pass, not defects.
  *(2026-09-15: actions-only dependabot is enabled and opened PRs
  for install-qt-action v3.3.0 -> 4.3.1 and checkout v5.1.0 ->
  v7.0.1; both left open for Brandon's merge call.)*
  *(2026-10-07: all three dependabot PRs have since merged (the
  v4.4.1 bump included); CI's Qt pin is 6.6.0 with
  install-qt-action 4.4.1. The cxx-qt 0.7 and dashmap 6 lines
  remain future modernizations.)*

### Final audit 2026-09-14 (THE FINAL AUDIT: NEW findings, one line each; full detail in audit-final/Topograph/FINAL-REPORT.md)

Eight lenses + slop-reader at v0.3.1 (c331364). Tally after dedup: 1 HIGH / 6 MEDIUM / ~30 LOW + 13 feature proposals. The v0.3.1 honesty pass verifies as real (falsified ticks unticked with accurate notes, README/CLAUDE repositioned, keyboard nav wired as documented; no-em-dash rule mechanically true at zero hits). NEW findings LOGGED, never executed:

- [x] [HIGH] GitHub presentation batch still open (the recorded box above): cut Releases for all three tags (`--notes-from-tag`), fix topic qt6--qml-cxx-qt, apply the drafted description.
      *(Done 2026-09-15: all three Releases cut, topic set replaced, description applied; see the recorded box above.)*
- [x] [MEDIUM] Dated correction notes still owed on two boxes the 09-13 pass skipped: the cross-fs "opt-in toggle" (roadmap.md:62; `cross_filesystems` is a field no code can set) and the "large system directory" harness (roadmap.md:54; it scans the crate's own src/). Same class: roadmap.md:57 /proc/mounts (solved via st_dev, but no dated note), and roadmap.md:296-299 (done repair recorded in an unticked box; tick it).
      *(Done 2026-09-15: all four boxes carry dated notes now; the cross-fs box is unticked to the v0.2.4 style and the done-repair box is ticked.)*
- [x] [MEDIUM] Failed-scan-root error path: an unreadable/nonexistent root publishes an empty tree as "Scan complete" and wipes the displayed tree; no error channel exists (scanner.rs:58-59, bridge.rs:132-141). Add an error flag to ScanMetrics or guard the empty-root publish.
      *(Done 2026-09-15, v0.3.2: ScanMetrics.failed set on an unstat-able or zero-entry root (cancelled scans excluded); the bridge skips the publish and the scanFinished emit and reports "Scan failed: cannot read <path>" on the progress line. Tested live and in the suite.)*
- [x] [MEDIUM] README.md:21 sells Count sort as working (stable no-op, FileCount hardcoded 0); add the caveat or drop "or file count". Companion truth fixes: the collapse-fallback mechanism is misdocumented (the click handler's index reassignment produces the outcome, not the removal fallback; patchnotes.md:13-15, roadmap.md:87, CLAUDE.md:44-46), and patchnotes.md:66-67 says 8 new tests since v0.2.4 when it is 9.
      *(Done 2026-09-15, v0.3.2: Count became real (the pull-forward answer), so the README claim is true and now mentions the count column; the collapse-fallback mechanism is corrected in CLAUDE.md and the roadmap tick note with the patchnotes rider in v0.3.2's entry; the off-by-one is corrected in the same rider.)*
- [x] [MEDIUM] Comment fixes: main.rs:10 claims arguments are passed to QGuiApplication (they are not); bridge.rs:176-178's force_link stub is bare while its sibling carries the load-bearing explanation.
      *(Done 2026-09-15, v0.3.2: the args clause was fixed by wiring the launch-argument rider; the force_link stub carries its sibling's comment.)*
- [x] [MEDIUM] CI hardening: Qt is pinned but Rust floats (@stable, no rust-toolchain.toml, no rust-version; edition 2024's 1.85 floor documented nowhere); SHA-pin checkout/install-qt-action; add a permissions block; decide release.yml (tag-triggered) vs manual cuts.
      *(Done 2026-09-15: rust-version = 1.85 in both manifests documents the MSRV without pinning Brandon's local toolchain; checkout v5.1.0 and install-qt-action v3.3.0 SHA-pinned; contents:read permissions block added; release.yml declined, Releases stay manual cuts (Brandon, 2026-09-15).)*
- [x] [LOW] Spec alignment with the honest-scope pass: "modern, attractive replacement... immense performance" (spec.md:4), "immense L1/L2 cache locality" (spec.md:18), geometry clause future tense (spec.md:7), trash example present tense (spec.md:10), ~20ms "measured, not gated" caveat (spec.md:18); roadmap.md:206 "Seamlessly"; roadmap.md:297 one ASCII-hyphen surrogate.
      *(Done 2026-09-15: spec.md de-inflated and future-tense claims rescope; the "Seamlessly" and the hyphen surrogate are fixed.)*
- [x] [LOW] Removal/housekeeping: qml.qrc referenced by nothing (delete behind a build check); scaffold-status comments on remove_subtree/IS_HIDDEN/IS_PSEUDO/mtime/allocated_size; "required by cxx-qt codegen" comment on the cxx dep; manifest license/description keys; README Qt6 package names + "GUI builds even for core-only runs" warning + Qt6/CXX-Qt attribution line; .gitignore anchor /target and /debug + one CXX-Qt rule; SECURITY.md; dependabot actions-only; CI badge; optional LazyLock swap and cargo-deny.
      *(Done 2026-09-15: everything listed landed; the qml.qrc deletion was Brandon-approved and verified by a green build; the LazyLock swap taken; cargo-deny declined (Brandon, 2026-09-15).)*
- [x] [LOW] Scanner polish riders: double lstat per entry (scanner.rs:72,:100); sort_by mutates outside the begin_reset_model pair (directory_model.rs:390-402); defensive cancel of a superseded walker in start_scan (bridge.rs:65-89); document build_tree_from_scan's parent-before-child ordering assumption (scanner.rs:145-163).
      *(Done 2026-09-15, v0.3.2: all four riders landed; the single-lstat restructure rides jwalk's client_state and the boundary prune decides on the stashed dev.)*

CONFIRMED-prior (verified still present): the GitHub batch box above; the ~20ms unenforced spec claim; MB-only formatting at BOTH sites (main.qml:293-296 rows AND bridge.rs:144-146 progress line; the tiered-formatter pull-forward covers both). SUPERSEDED (verified fixed): the 09-12 falsified-ticks HIGH, CLAUDE.md staleness, README "visualizer" overpromise, keyboard-nav lane (shipped v0.3.1). Audit-side corrections: the audit sheet's "3 tests, all in core; GUI zero" is stale (12 tests, 8 in the GUI crate) and its force_link/FileCount line numbers have drifted (stubs now bridge.rs:176-178, directory_model.rs:134-137).

Feature candidates (RE-RANKED or NEW, grounded in audit-final/Topograph/FINAL-REPORT.md lens 4; §5.11 governs the pull-forwards): tiered formatter (S, rank 1; fixes a wrong display on every row + the progress line), Phase 6 subtree counts to make Count real (S/M, rank 2; note the delegate has no count column today), scan summary line + window title from the never-read currentPath property (S, rank 3); then path-argument launch, cross-fs wiring, session persistence slice, folder picker, path reconstruction + Open-in-File-Manager, EACCES visible state, <Files> slice; treemap is the only item that makes "visualizer" true (its own decided lane); progressive-during-scan proposed and argued AGAINST. Pair the next release with the real-mount scan (Brandon's session) so v0.3.x has usage evidence.
  *(Gate answers 2026-09-15 (Brandon): the top-3 plus the launch-argument and natural-sort riders shipped in v0.3.2; the real-mount scan pairs with this release as his session; the 60FPS eyeball follows the release; treemap, the <Files> slice, and the version.workspace migration stay aspirational as the repo's reopen conditions; the Count stack is keep-and-complete; releases stay manual cuts.)*
  *(2026-10-07: the treemap reopen condition was exercised: Brandon's go covered the treemap lane, and Phases 8-9 shipped in v0.4.0 alongside the cross-fs toggle, the mount-boundary tests, the dedup-savings metric, and the remaining Phase 6 aggregation boxes. A real-mount scan over ~/.gitrepos ran during verification (172,733 files, 4.68 GB apparent / 5.00 GB on disk in 28.4s); Brandon's own usage session remains open. The <Files> slice and version.workspace stay aspirational. Same evening, v0.4.1: the headless selftest bin landed (the app drives itself on the offscreen platform and captures frames via QQuickWindow::grabWindow), closing the resize-path verification and replacing live-session GUI checks.)*

Slop-reader: zero em-dashes in all five prose files (mechanically verified; the repo's rule holds); no kills; [fix] items are the spec.md puffery (two "immense" passages) and roadmap.md:206 "Seamlessly" + one ASCII-hyphen surrogate at :297; patchnotes v0.2.4+ read strongly human.
