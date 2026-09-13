# Patch Notes

## v0.3.1 (2026-09-13)

**Keyboard navigation, plus the record repairs from the 2026-09-12
decision session.**

- Keyboard navigation: the tree takes focus and shows a Kanagawa
  highlight on the current row. Up/Down move the selection (the
  ListView's built-in navigation), Left collapses and Right expands the
  selected directory, and the selection now survives every expand,
  collapse, and sort: the row's name and depth are snapshotted before
  the model reset and re-found afterwards, falling back to the row's
  old index when the operation removed it (collapsing a directory while
  a descendant is selected selects the collapsed parent). Row clicks
  and scan completion hand focus to the tree so the arrow keys work
  immediately. Verified by driving the live app with synthetic key
  input.
- Patchnotes rider for 2026-09-12: the aspirational marking of Phases
  6-20 + TUI and the retirement of the rustix, inode-sorting, and
  dedup-savings verdicts were recorded in the roadmap that evening but
  never announced; this entry is their record.
- Roadmap honesty from the six-lens audit: the mount-boundary
  integration test never existed (the st_dev pruning has no test
  coverage) and the only size formatter always prints MB, so both
  falsified ticks are unticked with dated notes. The tiered formatter
  remains available as a small pull-forward.
- Docs repositioned: CLAUDE.md no longer claims all rendering is
  GPU-accelerated (the shader phases are aspirational) and cites the
  aspirational flag alongside Phase 6; README now describes a size
  explorer with the treemap planned instead of a qdirstat-analogous
  visualizer.
- Scope notes recorded without action: CI's install-qt-action@v3 and
  the Qt 6.6.0 pin are aging, cxx-qt 0.6 has a 0.7.x line with API
  churn, and dashmap 5.5 has a 6.x line.

## v0.3.0 (2026-09-06)

**First feature release since the hygiene pass: the tree view becomes a
real tree.**

- Lazy expansion: directory rows expand and collapse in place. Model rows
  carry arena node IDs, so `expandRow`/`collapseRow` splice in or remove
  a row's direct children and only expanded levels exist as UI rows. A
  stale-click name check rejects node ids left over from a republished
  tree.
- Sorting: Size (default, descending), Name, and Count keys via a
  `sortBy` invokable and a Kanagawa sort header. Sibling runs reorder in
  place with their subtrees, newly expanded children follow the active
  sort, and Count is a stable no-op while the FileCount role stays dead
  until Phase 6 subtree counts exist.
- Inline percentage bars: a Percent role (row aggregate size over parent
  aggregate size) drawn as a Kanagawa-aqua bar with the numeric share in
  the size column.
- Cancel race closed: the shared tree slot is generation-guarded, so a
  cancelled or superseded scan finishing late can no longer overwrite a
  newer scan's tree (audit finding; regression test included).
- Model refresh repaired: the string-keyed QML check
  (`progressText === "Scan complete."` inside `onIsScanningChanged`)
  never fired because the bridge wrote `is_scanning` before the text.
  The bridge now emits a `scanFinished` signal on completion and QML
  reloads the model from it; found and verified with a live run of the
  app.
- Phase 6 honesty pass: boxes duplicating already-shipped work are ticked
  with dated notes, and a known-hierarchy aggregation test now covers
  intermediate directory sums in topograph-core (12 tests total, 8 of
  them new since v0.2.4; the GUI crate carries its first tests).
- Docs: patchnotes structure repaired (stray second H1, doubled v0.2.3
  prefix, mixed heading styles, missing dates), roadmap header count
  corrected to 23 groups, Phase 0 note untangled, "an known tree" typo
  fixed, and the em-dash the prose rule forbids removed.

## v0.2.4 (2026-09-04)

**Hygiene release from the workspace audit's Stage 0 pass.** No behavior
change.

- Version surfaces now agree at 0.2.4: the member manifests and the stale
  `Cargo.lock` were regenerated in the release commit, and the `VERSION`
  file, mistakenly left at 0.2.3 there, is corrected in this repair commit.
  The dead GTK-era `[workspace.dependencies]`
  (gtk4, cairo-rs, tokio; uninherited leftovers of the abandoned GUI
  framework) are removed, along with the decorative `[workspace.package]`
  version key.
- The release commit failed the newly wired `clippy -D warnings` gate (an
  unused test-only `File` import); the fix (`1ae0469`) rides in the tagged
  tree. Roadmap ticks that the workspace audit found falsified (savings
  metric, percentage bar, lazy expansion) are unticked with notes, the
  silently shipped model invalidation is ticked, and the spec's `NodeId`
  description now matches indextree's actual index type.
- CI now runs `cargo fmt --check` and `cargo clippy -D warnings` beside the
  tests (the roadmap claimed this; it wasn't wired).
- README corrected: the app is Qt6/QML via CXX-Qt, not GTK4, and the build
  instructions are real.
- Roadmap: the two duplicate "Phase 1" groups retitled, Phase 0's absorbed
  duplicates closed, the falsified rustix/inode boxes unticked with notes,
  and the `../Lattice` reference corrected.

## v0.2.3 (2026-08-23)

- **Build:** add GitHub Actions Qt6 CI workflow

## v0.2.2 (2026-08-15)
### Changed
- Converted the entire QML UI to the **Kanagawa Dragon** colour scheme. Replaced placeholder colours with exact hex values parsed from desktop configurations (`#181616` background, `#282727` surface, `#c5c9c5` foreground, `#625e5a` muted text, with Dragon Red and Green accents).

## v0.2.1 (2026-08-15)
### Added
- **Phase 5**: Hooked the backend `FileTree` into QML via a CXX-Qt `DirectoryModel` (`QAbstractListModel`).
- The `ScanBridge` now notifies the UI when a scan completes, automatically triggering `DirectoryModel::load_tree` to fetch the new filesystem hierarchy from a shared thread-safe lock.
- Added a `ListView` (tree layout placeholder) in `main.qml` to render the root contents of the file system dynamically using standard Qt declarative delegates.

### Fixed
- Fixed CXX-Qt build system regressions where multiple `#[cxx_qt::bridge]` modules with identical names silently overrode each other, stripping `Q_PLUGIN_METADATA` and resulting in missing QML types.
- Fixed linker stripping issues by exposing explicit `force_link` stubs to ensure static CXX-Qt initializers execute before the QML engine initializes.
- Removed unused imports and mutable warnings in `topograph-core`.

## v0.2.0 (2026-08-14)
- **Memory Architecture**: Implemented cache-friendly `indextree` arena and `NodeData` for zero-allocation tree structures.
- **Concurrent Scanning**: Integrated `jwalk` and `crossbeam-channel` for highly parallelized directory traversal.
- **Deduplication**: Added $O(1)$ hardlink deduplication via `DashSet` and device boundary pruning.
- **Testing**: Added rigorous unit tests ensuring the arena supports 1,000,000 nodes, and hardlinks deduplicate correctly. All tests passing.

## v0.1.0 (2026-08-14)
- Initial skeleton and scaffolding for Topograph.
