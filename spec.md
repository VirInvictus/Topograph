# Topograph Spec

## Purpose
Topograph is a native application for visualizing file system usage. It serves as a modern, attractive replacement for tools like qdirstat or WinDirStat, built specifically for Linux desktops, prioritizing immense performance (Rust) and aesthetics (Qt/QML).

## Semantics
- **Headless Engine**: `topograph-core` handles all directory scanning, size aggregation, and geometry layout algorithms, completely decoupled from the UI.
- **UI Layer**: `topograph` provides a Qt6 / QML shell driven by `cxx-qt`, styled with the Kanagawa Dragon theme.
- **Visualizations**: Both Squarified Cushion Treemaps and Radial Sunburst Charts will be supported, calculated mathematically and rendered directly via hardware-accelerated shaders or `QSGGeometryNode` to bypass traditional QML object overhead.
- **State**: The application does not write or mutate the filesystem by default, aside from specific opt-in actions (e.g., "move to trash") triggered manually by the user.
