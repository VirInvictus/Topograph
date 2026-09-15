# CLAUDE.md

## Topograph

A fast, local-first file system size explorer; the treemap visualization is planned.

**Language:** Rust 2024
**Framework:** Qt6 / QML (via CXX-Qt)

- Build: `cargo build`
- Run: `cargo run -p topograph`
- Tests: `cargo test`

Note: This project relies on Kanagawa Dragon for its styling. What ships today is a plain QML ListView over the Rust core; the GPU-shader rendering phases (treemap, sunburst) are aspirational scope, marked as such at roadmap.md:4, not shipped. No libadwaita or GTK logic exists here anymore.

## Model notes

- `DirectoryModel` is a flat `Vec<NodeDisplay>` under one root index; rows carry
  arena `NodeId`s. `expandRow`/`collapseRow` splice in or remove a row's direct
  children and publish with a full model reset, so only expanded levels exist
  as rows. A collapsed row owns every following row deeper than itself.
- Sorting lives on the model: `sortBy(key, descending)` (QML header) reorders
  each sibling run with `sort_range`, whole subtrees moving with their parents.
  New children from `expandRow` inherit the active sort. Keys: size (default,
  descending), name, count; count is a stable no-op while FileCount is dead.
- The Percent role is the row's aggregate size over its parent's aggregate
  size, computed at row-build time in the model (not stored in the arena);
  the QML delegate draws it as an inline bar in the size column.
- `LATEST_TREE` (bridge.rs) is the shared scan-result slot. `loadTree` rebuilds
  from it when the bridge emits `scanFinished` (from `update_metrics` once the
  worker reports completion). The old mechanism keyed on the UI string
  `progressText === "Scan complete."` and never fired: `is_scanning` flipped
  before the text was written, so the handler always read the stale value.
  Publication is generation-guarded (`TREE_GENERATION` in bridge.rs): starting
  or cancelling a scan invalidates the in-flight worker's late publish.
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
- The FileCount role is always 0 until per-subtree counts exist (Phase 6 is
  aspirational scope; see roadmap.md:4).
