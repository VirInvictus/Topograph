# CLAUDE.md

## Topograph

A blazing fast file system visualizer.

**Language:** Rust 2024
**Framework:** Qt6 / QML (via CXX-Qt)

- Build: `cargo build`
- Run: `cargo run -p topograph`
- Tests: `cargo test`

Note: This project relies on Kanagawa Dragon for its styling. All rendering is GPU-accelerated and strictly separated from the Rust core. No libadwaita or GTK logic exists here anymore.

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
  from it and is keyed on the UI string `progressText === "Scan complete."`
  (main.qml): keep that string exact or replace the mechanism deliberately.
  Publication is generation-guarded (`TREE_GENERATION` in bridge.rs): starting
  or cancelling a scan invalidates the in-flight worker's late publish.
- The `force_link()` stubs (bridge.rs, directory_model.rs) are load-bearing
  CXX-Qt 0.6 anti-stripping shims. Never delete them.
- The FileCount role is always 0 until per-subtree counts exist (Phase 6).
