# Topograph Spec

## Purpose
Topograph is a native application for exploring file system usage. It aims to be a fast, attractive alternative to tools like qdirstat or WinDirStat on Linux desktops, prioritizing performance (Rust) and aesthetics (Qt/QML).

## Semantics
- **Headless Engine**: `topograph-core` handles all directory scanning and size aggregation, completely decoupled from the UI; the geometry layout algorithms for the planned visualizations (Phases 8/10) will live here too.
- **UI Layer**: `topograph` provides a Qt6 / QML shell driven by `cxx-qt`, styled with the Kanagawa Dragon theme.
- **Visualizations**: Both Squarified Cushion Treemaps and Radial Sunburst Charts will be supported, calculated mathematically and rendered directly via hardware-accelerated shaders or `QSGGeometryNode` to bypass traditional QML object overhead.
- **State**: The application does not write or mutate the filesystem; the only planned writes are specific opt-in actions (e.g., a future "move to trash", Phase 16) triggered manually by the user.
- **Tree list**: the GUI exposes the scanned tree as a flat `QAbstractListModel` (`DirectoryModel`) whose rows carry arena `NodeId`s; directories expand and collapse lazily by splicing or removing their direct children, so only expanded levels exist as rows.

## Memory Architecture
Topograph uses a cache-friendly flat arena (backed by `indextree`) to model the file system graph. This prevents heap fragmentation and pointer-chasing associated with traditional C++ `shared_ptr` or `Box<Node>` trees.
- `NodeId`: indextree's pointer-sized index (`NonZeroUsize`) plus a reuse stamp; not a 32-bit value.
- `NodeData`: Compact payload containing names, allocated sizes, and metadata bitflags (`NodeFlags`).
- **Concurrent Building**: The arena is populated safely by streaming node results from the multi-threaded file scanner.
- **Aggregation**: Subtree sizing is aggregated post-order in $O(N)$ time with good cache locality; the million-node synthetic test logs ~20ms in release mode (measured, not gated; debug builds and CI machines vary).
