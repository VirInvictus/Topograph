# CLAUDE.md

## Topograph

A blazing fast file system visualizer.

**Language:** Rust 2024
**Framework:** Qt6 / QML (via CXX-Qt)

- Build: `cargo build`
- Run: `cargo run -p topograph`
- Tests: `cargo test`

Note: This project relies on Kanagawa Dragon for its styling. All rendering is GPU-accelerated and strictly separated from the Rust core. No libadwaita or GTK logic exists here anymore.
