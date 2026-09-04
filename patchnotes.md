# Patch Notes

## v0.2.4 (2026-09-04)

**Hygiene release from the workspace audit's Stage 0 pass.** No behavior
change.

- Version drift closed: VERSION 0.2.3, both member manifests now 0.2.4, the
  stale `Cargo.lock` regenerated. The dead GTK-era `[workspace.dependencies]`
  (gtk4, cairo-rs, tokio — uninherited leftovers of the abandoned GUI
  framework) are removed, along with the decorative `[workspace.package]`
  version key.
- CI now runs `cargo fmt --check` and `cargo clippy -D warnings` beside the
  tests (the roadmap claimed this; it wasn't wired).
- README corrected: the app is Qt6/QML via CXX-Qt, not GTK4, and the build
  instructions are real.
- Roadmap: the two duplicate "Phase 1" groups retitled, Phase 0's absorbed
  duplicates closed, the falsified rustix/inode boxes unticked with notes,
  and the `../Lattice` reference corrected.

## v0.2.3 (2026-08-23)

- **Build:** build: add GitHub Actions Qt6 CI workflow

# Patch Notes

## [0.2.2] - 2026-08-15
### Changed
- Converted the entire QML UI to the **Kanagawa Dragon** colour scheme. Replaced placeholder colours with exact hex values parsed from desktop configurations (`#181616` background, `#282727` surface, `#c5c9c5` foreground, `#625e5a` muted text, with Dragon Red and Green accents).

## [0.2.1] - 2026-08-15
### Added
- **Phase 5**: Hooked the backend `FileTree` into QML via a CXX-Qt `DirectoryModel` (`QAbstractListModel`).
- The `ScanBridge` now notifies the UI when a scan completes, automatically triggering `DirectoryModel::load_tree` to fetch the new filesystem hierarchy from a shared thread-safe lock.
- Added a `ListView` (tree layout placeholder) in `main.qml` to render the root contents of the file system dynamically using standard Qt declarative delegates.

### Fixed
- Fixed CXX-Qt build system regressions where multiple `#[cxx_qt::bridge]` modules with identical names silently overrode each other, stripping `Q_PLUGIN_METADATA` and resulting in missing QML types.
- Fixed linker stripping issues by exposing explicit `force_link` stubs to ensure static CXX-Qt initializers execute before the QML engine initializes.
- Removed unused imports and mutable warnings in `topograph-core`.

## v0.2.0
- **Memory Architecture**: Implemented cache-friendly `indextree` arena and `NodeData` for zero-allocation tree structures.
- **Concurrent Scanning**: Integrated `jwalk` and `crossbeam-channel` for highly parallelized directory traversal.
- **Deduplication**: Added $O(1)$ hardlink deduplication via `DashSet` and device boundary pruning.
- **Testing**: Added rigorous unit tests ensuring the arena supports 1,000,000 nodes, and hardlinks deduplicate correctly. All tests passing.

## v0.1.0
- Initial skeleton and scaffolding for Topograph.
