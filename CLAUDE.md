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
- `LATEST_TREE` (bridge.rs) is the shared scan-result slot. `loadTree` rebuilds
  from it and is keyed on the UI string `progressText === "Scan complete."`
  (main.qml): keep that string exact or replace the mechanism deliberately.
- The `force_link()` stubs (bridge.rs, directory_model.rs) are load-bearing
  CXX-Qt 0.6 anti-stripping shims. Never delete them.
- The FileCount role is always 0 until per-subtree counts exist (Phase 6).
