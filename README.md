# Topograph

A native Qt6/QML file system size explorer: fast, local-first, and styled with Kanagawa Dragon. The qdirstat-style treemap visualization is planned but not built yet.

## Building

Requires Qt6 (base + declarative) and a recent stable Rust toolchain.

```sh
cargo build --release
./target/release/topograph
```

The scanner core (`topograph-core`) is pure Rust with no Qt dependency; only
the GUI shell links Qt6 via CXX-Qt.

## Using

Hit Scan on a directory and the tree lists its contents. Directories expand
and collapse on click, so only the levels you open are materialized, the
header sorts any level by size, name, or file count, and every row shows its
share of the parent's size as an inline bar.

The tree is keyboard navigable: Up/Down move the selection, Left collapses
and Right expands the selected directory, and the selection follows the row
through expanding, collapsing, and re-sorting.

## License

MIT
