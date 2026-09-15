# Topograph

[![CI](https://github.com/VirInvictus/Topograph/actions/workflows/ci.yml/badge.svg)](https://github.com/VirInvictus/Topograph/actions/workflows/ci.yml)

A native Qt6/QML file system size explorer: fast, local-first, and styled with Kanagawa Dragon. The qdirstat-style treemap visualization is planned but not built yet.

## Building

Requires Qt6 (base + declarative) and a Rust toolchain of 1.85 or newer (the edition 2024 floor). On Fedora the Qt packages are `qt6-qtbase-devel` and `qt6-qtdeclarative-devel`; on Debian/Ubuntu, `qt6-base-dev` and `qt6-declarative-dev`. Note that even core-only and test runs build the GUI crate, so the Qt development packages are needed for `cargo test` too.

```sh
cargo build --release
./target/release/topograph
```

Launch with an optional path argument to seed the scan field: `./target/release/topograph /var`.

The scanner core (`topograph-core`) is pure Rust with no Qt dependency; only
the GUI shell links Qt6 via CXX-Qt.

## Using

Hit Scan on a directory and the tree lists its contents. Directories expand
and collapse on click, so only the levels you open are materialized, the
header sorts any level by size, name, or item count, and every directory row
shows its item count, its human-readable size, and its share of the parent's
size as an inline bar. A missing or unreadable scan root reports "Scan
failed" and leaves the displayed tree untouched.

The tree is keyboard navigable: Up/Down move the selection, Left collapses
and Right expands the selected directory, and the selection follows the row
through expanding, collapsing, and re-sorting.

## License

MIT. Topograph links Qt6 dynamically (LGPL-3.0) and bridges to it with
CXX-Qt (MIT OR Apache-2.0).
