# Patchnotes

## v0.2.0
- **Memory Architecture**: Implemented cache-friendly `indextree` arena and `NodeData` for zero-allocation tree structures.
- **Concurrent Scanning**: Integrated `jwalk` and `crossbeam-channel` for highly parallelized directory traversal.
- **Deduplication**: Added $O(1)$ hardlink deduplication via `DashSet` and device boundary pruning.
- **Testing**: Added rigorous unit tests ensuring the arena supports 1,000,000 nodes, and hardlinks deduplicate correctly. All tests passing.

## v0.1.0
- Initial skeleton and scaffolding for Topograph.
