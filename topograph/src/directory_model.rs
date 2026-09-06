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
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[base = "QAbstractListModel"]
        #[qml_element]
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
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use topograph_core::{FileTree, NodeFlags, NodeId};

pub struct NodeDisplay {
    pub node_id: NodeId,
    pub file_name: String,
    pub file_size: u64,
    pub file_count: usize,
    pub is_directory: bool,
    pub depth: u32,
    pub expanded: bool,
}

#[derive(Default)]
pub struct DirectoryModelRust {
    pub(crate) items: Vec<NodeDisplay>,
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
}

/// Builds the display rows for the direct children of `parent`, one level deep.
fn child_rows(tree: &FileTree, parent: NodeId, depth: u32) -> Vec<NodeDisplay> {
    tree.get_children(parent)
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
            }
        })
        .collect()
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

impl dir_model::DirectoryModel {
    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        roles.insert(Roles::FileName as i32, QByteArray::from("fileName"));
        roles.insert(Roles::FileSize as i32, QByteArray::from("fileSize"));
        roles.insert(Roles::FileCount as i32, QByteArray::from("fileCount"));
        roles.insert(Roles::IsDirectory as i32, QByteArray::from("isDirectory"));
        roles.insert(Roles::Depth as i32, QByteArray::from("depth"));
        roles.insert(Roles::Expanded as i32, QByteArray::from("expanded"));
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
            }
        }
        QVariant::default()
    }

    pub fn load_tree(mut self: Pin<&mut Self>) {
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
                });
                new_items.extend(child_rows(tree, root_id, 1));
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
                    child_rows(tree, node_id, child_depth)
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

    fn row(node_id: NodeId, name: &str, depth: u32, is_directory: bool) -> NodeDisplay {
        NodeDisplay {
            node_id,
            file_name: name.to_string(),
            file_size: 0,
            file_count: 0,
            is_directory,
            depth,
            expanded: false,
        }
    }

    #[test]
    fn child_rows_lists_one_level() {
        let (tree, root, ..) = sample_tree();
        let rows = child_rows(&tree, root, 1);

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
            row(root, "root", 0, true),
            row(a, "a", 1, true),
            row(a1, "a-1", 2, true),
            row(deep, "deep.txt", 3, false),
            row(b, "b.txt", 1, false),
        ];

        assert_eq!(descendant_end(&rows, 0), 5); // root owns all
        assert_eq!(descendant_end(&rows, 1), 4); // a owns a-1 and deep.txt
        assert_eq!(descendant_end(&rows, 2), 4); // a-1 owns deep.txt
        assert_eq!(descendant_end(&rows, 4), 5); // leaf owns nothing
    }
}
