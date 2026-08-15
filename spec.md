# Topograph Spec

## Purpose
Topograph is a native application for visualizing file system usage. It serves as a modern, attractive replacement for tools like qdirstat or WinDirStat, built specifically for Linux desktops, prioritizing immense performance (Rust) and aesthetics (Qt/QML).

## Semantics
- **Headless Engine**: `topograph-core` handles all directory scanning, size aggregation, and geometry layout algorithms, completely decoupled from the UI.
- **UI Layer**: `topograph` provides a Qt6 / QML shell driven by `cxx-qt`, styled with the Kanagawa Dragon theme.
- **Visualizations**: Both Squarified Cushion Treemaps and Radial Sunburst Charts will be supported, calculated mathematically and rendered directly via hardware-accelerated shaders or `QSGGeometryNode` to bypass traditional QML object overhead.
- **State**: The application does not write or mutate the filesystem by default, aside from specific opt-in actions (e.g., "move to trash") triggered manually by the user.

## Memory Architecture
Topograph uses a cache-friendly flat arena (backed by `indextree`) to model the file system graph. This prevents heap fragmentation and pointer-chasing associated with traditional C++ `shared_ptr` or `Box<Node>` trees.
- `NodeId`: A lightweight 32-bit index into the contiguous arena.
- `NodeData`: Compact payload containing names, allocated sizes, and metadata bitflags (`NodeFlags`).
- **Concurrent Building**: The arena is populated safely by streaming node results from the multi-threaded file scanner.
- **Aggregation**: Subtree sizing is aggregated post-order in $O(N)$ time with immense L1/L2 cache locality, completing in ~20ms for 1,000,000 nodes.
