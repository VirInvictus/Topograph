# Topograph Roadmap

The 20-phase master plan synthesized from `qdirstat`, `filelight`, and `baobab`.

- [x] Phase 0: Project skeleton, CXX-Qt bindings setup, and Kanagawa Dragon QML integration.
- [ ] Phase 1: **Memory Architecture**: Implement a cache-friendly Arena / Vector-indexed graph (`indextree` or flat `Vec`) for the file tree, eliminating heap fragmentation and pointer-chasing.
- [ ] Phase 2: **Concurrent Scanning Engine**: Implement work-stealing parallel directory traversal (`rayon` or `jwalk`) using `openat` and `fstatat` with `AT_SYMLINK_NOFOLLOW`.
- [ ] Phase 3: **Deduplication & Boundaries**: Hardlink dedup gate (`st_nlink > 1` + Sharded HashMap) and `/proc/mounts` validation to prevent external mount traversal.
- [ ] Phase 4: **Atomic UI Integration**: Lock-free progress reporting via `AtomicU64` counters, polled by a 60FPS QML timer (zero IPC overhead/signal flooding).
- [ ] Phase 5: **Tree List UI**: Rust-to-QML ListModel bridge for the side pane navigation.
- [ ] Phase 6: **Aggregation Math**: Post-order recursive size accumulation, item counts, and UI percentage bars.
- [ ] Phase 7: **Pseudo-nodes**: Implement `<Files>` and `<Ignored>` grouping nodes (QDirStat concept) for clean directory-vs-files comparisons.
- [ ] Phase 8: **Squarified Treemap Layout**: Fast math layer computing rectangle bounds using the Bruls sub-threshold packing algorithm.
- [ ] Phase 9: **Cushion Treemap Rendering**: Hardware-accelerated parabolic shading via QML ShaderEffect / GPU Fragment Shader (replacing QDirStat's CPU rendering).
- [ ] Phase 10: **Sunburst Layout Math**: Radial arc geometry calculation using integer polar constraints (Filelight's 16th-degree arithmetic) and Baobab's depth culling limits.
- [ ] Phase 11: **Sunburst Rendering**: Single `QSGGeometryNode` for the entire ring chart, preventing thousands of QML `Shape` items from crashing the compositor.
- [ ] Phase 12: **O(1) Interaction**: Hit-testing based on math (AABB for treemaps, polar `atan2` for rings) instead of iterating over UI rectangles for hover/tooltip performance.
- [ ] Phase 13: **Color Mapping**: Map the Kanagawa Dragon palette dynamically to directory depth, file type, and saturation scales.
- [ ] Phase 14: **Drill-down Navigation**: Clicking a wedge or block to instantly refocus the root of the visualization.
- [ ] Phase 15: **Subtree Caching**: In-memory caching logic allowing instant return to parent paths without rescanning the disk.
- [ ] Phase 16: **Action Interactivity**: Right-click context menus (Open in file manager, Move to Trash, Open Terminal).
- [ ] Phase 17: **Filter Pipeline**: Exclude paths, regex filters, and dynamic minimum visual size thresholding.
- [ ] Phase 18: **State Persistence**: QSettings hookups for window sizing, view preferences, and debouncing resize recalculations.
- [ ] Phase 19: **Optimization & Profiling**: Memory profiling on millions of small files, benchmarking against QDirStat.
- [ ] Phase 20: **1.0 Release**: Documentation, Flatpak/AppImage packaging, and polished screenshots.
