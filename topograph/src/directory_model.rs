#[cxx_qt::bridge]
pub mod dir_model {
    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;

        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;

        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[base = "QAbstractListModel"]
        #[qml_element]
        #[qproperty(QString, sort_key)]
        #[qproperty(bool, sort_descending)]
        type DirectoryModel = super::DirectoryModelRust;

        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut DirectoryModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );

        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut DirectoryModel>);

        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut DirectoryModel>);

        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut DirectoryModel>);
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &DirectoryModel) -> QHash_i32_QByteArray;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &DirectoryModel, _parent: &QModelIndex) -> i32;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "data"]
        fn data(self: &DirectoryModel, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_name = "loadTree"]
        fn load_tree(self: Pin<&mut DirectoryModel>);

        #[qinvokable]
        #[cxx_name = "expandRow"]
        fn expand_row(self: Pin<&mut DirectoryModel>, row: i32);

        #[qinvokable]
        #[cxx_name = "collapseRow"]
        fn collapse_row(self: Pin<&mut DirectoryModel>, row: i32);

        #[qinvokable]
        #[cxx_name = "sortBy"]
        fn sort_by(self: Pin<&mut DirectoryModel>, key: QString, descending: bool);
    }
}

use core::pin::Pin;
use std::cmp::Ordering;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use topograph_core::{FileTree, NodeFlags, NodeId};

#[derive(Clone)]
pub struct NodeDisplay {
    pub node_id: NodeId,
    pub file_name: String,
    pub file_size: u64,
    pub file_count: usize,
    pub is_directory: bool,
    pub depth: u32,
    pub expanded: bool,
    /// This node's share of its parent's aggregated size, 0-100.
    pub percent: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Size,
    Name,
    Count,
}

#[derive(Clone, Copy, Debug)]
pub struct SortState {
    pub key: SortKey,
    pub descending: bool,
}

pub struct DirectoryModelRust {
    pub(crate) items: Vec<NodeDisplay>,
    pub(crate) sort_key: QString,
    pub(crate) sort_descending: bool,
}

impl Default for DirectoryModelRust {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            sort_key: QString::from("size"),
            sort_descending: true,
        }
    }
}

pub fn force_link() {
    // Force linking of the C++ object file by referencing a C++ method
    let _ = dir_model::DirectoryModel::begin_reset_model as *const ();
}

#[repr(i32)]
pub enum Roles {
    FileName = 0x0100, // Qt::UserRole
    FileSize = 0x0101,
    FileCount = 0x0102,
    IsDirectory = 0x0103,
    Depth = 0x0104,
    Expanded = 0x0105,
    Percent = 0x0106,
}

/// A node's share of its parent's aggregated size, in percent. A zero parent
/// (an empty directory) leaves every child at zero rather than dividing.
fn percent_of(size: u64, parent_size: u64) -> f64 {
    if parent_size == 0 {
        0.0
    } else {
        (size as f64 / parent_size as f64) * 100.0
    }
}

/// Maps a QML-facing sort key name onto its enum value.
fn sort_key_from_qstring(key: &QString) -> Option<SortKey> {
    match key.to_string().as_str() {
        "size" => Some(SortKey::Size),
        "name" => Some(SortKey::Name),
        "count" => Some(SortKey::Count),
        _ => None,
    }
}

/// The sort currently configured on the model, as pure data.
fn sort_state(rust: &DirectoryModelRust) -> SortState {
    SortState {
        key: sort_key_from_qstring(&rust.sort_key).unwrap_or(SortKey::Size),
        descending: rust.sort_descending,
    }
}

fn compare_rows(a: &NodeDisplay, b: &NodeDisplay, sort: SortState) -> Ordering {
    let ord = match sort.key {
        SortKey::Size => a.file_size.cmp(&b.file_size),
        SortKey::Name => a.file_name.cmp(&b.file_name),
        SortKey::Count => a.file_count.cmp(&b.file_count),
    };
    if sort.descending { ord.reverse() } else { ord }
}

/// Builds the display rows for the direct children of `parent`, one level deep.
fn child_rows(tree: &FileTree, parent: NodeId, depth: u32, sort: SortState) -> Vec<NodeDisplay> {
    let parent_size = tree.get_data(parent).map(|d| d.size).unwrap_or(0);
    let mut rows: Vec<NodeDisplay> = tree
        .get_children(parent)
        .map(|node_id| {
            let data = tree
                .get_data(node_id)
                .expect("child ids reference live nodes");
            NodeDisplay {
                node_id,
                file_name: data.name.to_string(),
                file_size: data.size,
                file_count: 0,
                is_directory: data.flags.contains(NodeFlags::IS_DIRECTORY),
                depth,
                expanded: false,
                percent: percent_of(data.size, parent_size),
            }
        })
        .collect();
    sort_range(&mut rows, 0, sort);
    rows
}

/// Flat end offset (exclusive) of the subtree rooted at `row`: every following
/// row deeper than `row` belongs to it, because children are always inserted
/// contiguously after their parent.
fn descendant_end(items: &[NodeDisplay], row: usize) -> usize {
    let depth = items[row].depth;
    let mut end = row + 1;
    while end < items.len() && items[end].depth > depth {
        end += 1;
    }
    end
}

/// Sorts the sibling runs in `rows[start..]` by `sort`, moving each whole
/// subtree block with its parent. `rows[start]` marks the run's depth; a run
/// is partitioned into blocks that start on a row of that depth and extend
/// over their deeper descendants, then each block's interior recurses.
fn sort_range(rows: &mut Vec<NodeDisplay>, start: usize, sort: SortState) {
    let end = rows.len();
    if end - start < 2 {
        return;
    }
    let depth = rows[start].depth;

    let mut blocks: Vec<Vec<NodeDisplay>> = Vec::new();
    let mut i = start;
    while i < end {
        let mut j = i + 1;
        while j < end && rows[j].depth > depth {
            j += 1;
        }
        blocks.push(rows[i..j].to_vec());
        i = j;
    }

    blocks.sort_by(|a, b| compare_rows(&a[0], &b[0], sort));

    for block in &mut blocks {
        sort_range(block, 1, sort);
    }

    rows.splice(start..end, blocks.into_iter().flatten());
}

impl dir_model::DirectoryModel {
    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        roles.insert(Roles::FileName as i32, QByteArray::from("fileName"));
        roles.insert(Roles::FileSize as i32, QByteArray::from("fileSize"));
        roles.insert(Roles::FileCount as i32, QByteArray::from("fileCount"));
        roles.insert(Roles::IsDirectory as i32, QByteArray::from("isDirectory"));
        roles.insert(Roles::Depth as i32, QByteArray::from("depth"));
        roles.insert(Roles::Expanded as i32, QByteArray::from("expanded"));
        roles.insert(Roles::Percent as i32, QByteArray::from("percent"));
        roles
    }

    pub fn row_count(&self, _parent: &QModelIndex) -> i32 {
        self.items.len() as i32
    }

    pub fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        if let Some(item) = self.items.get(index.row() as usize) {
            if role == Roles::FileName as i32 {
                return QVariant::from(&QString::from(&item.file_name));
            } else if role == Roles::FileSize as i32 {
                return QVariant::from(&(item.file_size as i64));
            } else if role == Roles::FileCount as i32 {
                return QVariant::from(&(item.file_count as i32));
            } else if role == Roles::IsDirectory as i32 {
                return QVariant::from(&item.is_directory);
            } else if role == Roles::Depth as i32 {
                return QVariant::from(&(item.depth as i32));
            } else if role == Roles::Expanded as i32 {
                return QVariant::from(&item.expanded);
            } else if role == Roles::Percent as i32 {
                return QVariant::from(&item.percent);
            }
        }
        QVariant::default()
    }

    pub fn load_tree(mut self: Pin<&mut Self>) {
        let sort = sort_state(self.rust());
        if let Ok(lock) = crate::bridge::LATEST_TREE.read()
            && let Some(tree) = lock.as_ref()
        {
            let mut new_items = Vec::new();

            if let Some(root_id) = tree.get_root() {
                let root_data = tree.get_data(root_id).unwrap();
                new_items.push(NodeDisplay {
                    node_id: root_id,
                    file_name: root_data.name.to_string(),
                    file_size: root_data.size,
                    file_count: 0,
                    is_directory: root_data.flags.contains(NodeFlags::IS_DIRECTORY),
                    depth: 0,
                    expanded: true,
                    percent: 100.0,
                });
                new_items.extend(child_rows(tree, root_id, 1, sort));
            }

            unsafe {
                self.as_mut().begin_reset_model();
                self.as_mut().rust_mut().items = new_items;
                self.as_mut().end_reset_model();
            }
        }
    }

    pub fn expand_row(mut self: Pin<&mut Self>, row: i32) {
        let idx = row.max(0) as usize;
        let (node_id, name, child_depth) = {
            let rust = self.rust();
            match rust.items.get(idx) {
                Some(item) if item.is_directory && !item.expanded => {
                    (item.node_id, item.file_name.clone(), item.depth + 1)
                }
                _ => return,
            }
        };

        let new_rows = match crate::bridge::LATEST_TREE.read() {
            // The tree slot may have been republished since this row was built
            // (a rescan completing while the click was in flight); the name
            // check rejects stale node ids instead of expanding a wrong node.
            Ok(lock) => match lock.as_ref() {
                Some(tree) if tree.get_data(node_id).is_some_and(|d| *d.name == *name) => {
                    child_rows(tree, node_id, child_depth, sort_state(self.rust()))
                }
                _ => return,
            },
            Err(_) => return,
        };

        unsafe {
            self.as_mut().begin_reset_model();
            {
                let mut rust_mut = self.as_mut().rust_mut();
                rust_mut.items[idx].expanded = true;
                rust_mut.items.splice(idx + 1..idx + 1, new_rows);
            }
            self.as_mut().end_reset_model();
        }
    }

    pub fn collapse_row(mut self: Pin<&mut Self>, row: i32) {
        let idx = row.max(0) as usize;
        let end = {
            let rust = self.rust();
            match rust.items.get(idx) {
                Some(item) if item.expanded => descendant_end(&rust.items, idx),
                _ => return,
            }
        };

        unsafe {
            self.as_mut().begin_reset_model();
            {
                let mut rust_mut = self.as_mut().rust_mut();
                rust_mut.items[idx].expanded = false;
                rust_mut.items.drain(idx + 1..end);
            }
            self.as_mut().end_reset_model();
        }
    }

    pub fn sort_by(mut self: Pin<&mut Self>, key: QString, descending: bool) {
        let Some(sort_key) = sort_key_from_qstring(&key) else {
            return;
        };

        // The generated setters, not bare field writes: only the setters
        // emit the NOTIFY signals the QML sort header binds against.
        self.as_mut().set_sort_key(key);
        self.as_mut().set_sort_descending(descending);

        {
            let mut rust_mut = self.as_mut().rust_mut();
            let sort = SortState {
                key: sort_key,
                descending,
            };
            sort_range(&mut rust_mut.items, 0, sort);
        }

        unsafe {
            self.as_mut().begin_reset_model();
            self.as_mut().end_reset_model();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use topograph_core::NodeData;

    fn dir(tree: &mut FileTree, parent: NodeId, name: &str) -> NodeId {
        tree.add_child(
            parent,
            NodeData::new(name, 0, 0, 0, NodeFlags::IS_DIRECTORY),
        )
    }

    fn file(tree: &mut FileTree, parent: NodeId, name: &str, size: u64) -> NodeId {
        tree.add_child(
            parent,
            NodeData::new(name, size, size, 0, NodeFlags::empty()),
        )
    }

    /// root/
    ///   a/
    ///     a-1/
    ///       deep.txt
    ///   b.txt
    fn sample_tree() -> (FileTree, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let mut tree = FileTree::new();
        let root = tree.set_root(NodeData::new("root", 0, 0, 0, NodeFlags::IS_DIRECTORY));
        let a = dir(&mut tree, root, "a");
        let a1 = dir(&mut tree, a, "a-1");
        let deep = file(&mut tree, a1, "deep.txt", 10);
        let b = file(&mut tree, root, "b.txt", 5);
        (tree, root, a, a1, deep, b)
    }

    fn row(node_id: NodeId, name: &str, size: u64, depth: u32, is_directory: bool) -> NodeDisplay {
        NodeDisplay {
            node_id,
            file_name: name.to_string(),
            file_size: size,
            file_count: 0,
            is_directory,
            depth,
            expanded: false,
            percent: 0.0,
        }
    }

    #[test]
    fn percent_of_handles_math_and_zero_parents() {
        assert_eq!(percent_of(25, 100), 25.0);
        assert_eq!(percent_of(100, 100), 100.0);
        assert_eq!(percent_of(0, 100), 0.0);
        assert_eq!(percent_of(50, 0), 0.0); // empty parent: no division
    }

    #[test]
    fn child_rows_compute_percent_against_the_parent() {
        let (tree, root, ..) = sample_tree();
        // Aggregation first, like the bridge does before publishing.
        let mut tree = tree;
        tree.aggregate_sizes();

        let rows = child_rows(
            &tree,
            root,
            1,
            SortState {
                key: SortKey::Name,
                descending: false,
            },
        );

        // The sample tree sums to 15 bytes: a/ holds 10, b.txt holds 5.
        assert_eq!(rows[0].file_name, "a");
        assert!((rows[0].percent - (10.0 / 15.0) * 100.0).abs() < 1e-9);
        assert_eq!(rows[1].file_name, "b.txt");
        assert!((rows[1].percent - (5.0 / 15.0) * 100.0).abs() < 1e-9);
    }

    #[test]
    fn child_rows_lists_one_level() {
        let (tree, root, ..) = sample_tree();
        let rows = child_rows(
            &tree,
            root,
            1,
            SortState {
                key: SortKey::Name,
                descending: false,
            },
        );

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].file_name, "a");
        assert!(rows[0].is_directory);
        assert_eq!(rows[0].depth, 1);
        assert!(!rows[0].expanded);
        assert_eq!(rows[1].file_name, "b.txt");
        assert!(!rows[1].is_directory);
        assert_eq!(rows[1].file_size, 5);
    }

    #[test]
    fn descendant_end_spans_nested_levels() {
        // Row layout after fully expanding the sample: root(0) a(1) a-1(2)
        // deep.txt(3) b.txt(1). A collapsed row owns everything after it that
        // is deeper than the row itself.
        let (_tree, root, a, a1, deep, b) = sample_tree();
        let rows = vec![
            row(root, "root", 0, 0, true),
            row(a, "a", 0, 1, true),
            row(a1, "a-1", 0, 2, true),
            row(deep, "deep.txt", 10, 3, false),
            row(b, "b.txt", 5, 1, false),
        ];

        assert_eq!(descendant_end(&rows, 0), 5); // root owns all
        assert_eq!(descendant_end(&rows, 1), 4); // a owns a-1 and deep.txt
        assert_eq!(descendant_end(&rows, 2), 4); // a-1 owns deep.txt
        assert_eq!(descendant_end(&rows, 4), 5); // leaf owns nothing
    }

    #[test]
    fn sort_range_orders_siblings_and_moves_subtrees() {
        // The ids are opaque to sorting; names and sizes carry the scenario.
        // Layout: root(0) big(1){big-inner(2)} small(1) mid(1){mid-inner(2)}.
        let (_tree, root, a, a1, deep, b) = sample_tree();
        let mut rows = vec![
            row(root, "root", 0, 0, true),
            row(a, "big", 30, 1, true),
            row(a1, "big-inner", 1, 2, false),
            row(deep, "small", 10, 1, false),
            row(b, "mid", 20, 1, true),
            row(b, "mid-inner", 2, 2, false),
        ];

        sort_range(
            &mut rows,
            0,
            SortState {
                key: SortKey::Size,
                descending: true,
            },
        );

        let names: Vec<&str> = rows.iter().map(|r| r.file_name.as_str()).collect();
        // Size descending, and each inner row stays directly under its parent.
        assert_eq!(
            names,
            ["root", "big", "big-inner", "mid", "mid-inner", "small"]
        );
    }

    #[test]
    fn sort_range_by_name_ascending() {
        let (_tree, root, a, a1, deep, b) = sample_tree();
        let mut rows = vec![
            row(root, "root", 0, 0, true),
            row(a, "zeta", 0, 1, true),
            row(a1, "zeta-inner", 0, 2, false),
            row(deep, "alpha", 0, 1, false),
            row(b, "mid", 0, 1, false),
        ];

        sort_range(
            &mut rows,
            0,
            SortState {
                key: SortKey::Name,
                descending: false,
            },
        );

        let names: Vec<&str> = rows.iter().map(|r| r.file_name.as_str()).collect();
        assert_eq!(names, ["root", "alpha", "mid", "zeta", "zeta-inner"]);
    }

    #[test]
    fn sort_range_by_count_is_a_stable_noop_while_counts_are_zero() {
        // FileCount is dead (always 0) until Phase 6 subtree counts exist;
        // a stable sort must then leave the insertion order untouched.
        let (_tree, root, a, a1, deep, b) = sample_tree();
        let mut rows = vec![
            row(root, "root", 0, 0, true),
            row(a, "b-first", 0, 1, false),
            row(a1, "a-second", 0, 1, false),
            row(deep, "c-third", 0, 1, false),
            row(b, "unused", 0, 1, false),
        ];
        let before: Vec<String> = rows.iter().map(|r| r.file_name.clone()).collect();

        sort_range(
            &mut rows,
            0,
            SortState {
                key: SortKey::Count,
                descending: true,
            },
        );

        let after: Vec<String> = rows.iter().map(|r| r.file_name.clone()).collect();
        assert_eq!(before, after);
    }
}
