# Patch Notes

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
