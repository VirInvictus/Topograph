# Topograph Spec

## Purpose
Topograph is a native application for exploring file system usage. It aims to be a fast, attractive alternative to tools like qdirstat or WinDirStat on Linux desktops, prioritizing performance (Rust) and aesthetics (Qt/QML).

## Semantics
- **Headless Engine**: `topograph-core` handles all directory scanning and size aggregation, completely decoupled from the UI; the treemap layout and cushion-shading math (Phase 8) live here, and the sunburst geometry (Phase 10) will join it when built.
- **UI Layer**: `topograph` provides a Qt6 / QML shell driven by `cxx-qt`, styled with the Kanagawa Dragon theme.
- **Visualizations**: the Squarified Cushion Treemap is rendered through a custom `QQuickItem`/`QSGGeometryNode` in C++ fed by a flat vertex buffer computed in Rust, bypassing per-tile QML object overhead; the cushion lighting (normal dotted against a fixed light, ambient plus clamped diffuse) is evaluated per vertex on an adaptively subdivided grid rather than in a fragment shader, because `.qsb` shader baking would require qtshadertools, which the CI Qt install does not carry. A Radial Sunburst chart remains planned.
- **State**: The application does not write or mutate the filesystem; the only planned writes are specific opt-in actions (e.g. a future "move to trash", Phase 16) triggered manually by the user.
- **Tree list**: the GUI exposes the scanned tree as a flat `QAbstractListModel` (`DirectoryModel`) whose rows carry arena `NodeId`s; directories expand and collapse lazily by splicing or removing their direct children, so only expanded levels exist as rows.

## Memory Architecture
Topograph uses a cache-friendly flat arena (backed by `indextree`) to model the file system graph. This prevents heap fragmentation and pointer-chasing associated with traditional C++ `shared_ptr` or `Box<Node>` trees.
- `NodeId`: indextree's pointer-sized index (`NonZeroUsize`) plus a reuse stamp; not a 32-bit value.
- `NodeData`: compact payload of names, sizes, and metadata bitflags (`NodeFlags`); the aggregation pass additionally writes per-subtree item counts, the node's share of its parent and its sibling-run offset (both percent, for the views), and the subtree's oldest/newest mtimes.
- **Concurrent Building**: The arena is populated safely by streaming node results from the multi-threaded file scanner.
- **Aggregation**: Subtree sizing is aggregated post-order in $O(N)$ time with good cache locality; the million-node synthetic test logs ~20ms in release mode (measured, not gated; debug builds and CI machines vary).
