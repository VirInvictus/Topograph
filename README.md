# Topograph

A native Qt6/QML file system visualizer analogous to qdirstat. Fast, local-first, and styled with Kanagawa Dragon.

## Building

Requires Qt6 (base + declarative) and a recent stable Rust toolchain.

```sh
cargo build --release
./target/release/topograph
```

The scanner core (`topograph-core`) is pure Rust with no Qt dependency; only
the GUI shell links Qt6 via CXX-Qt.

## License

MIT
