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
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};

pub struct NodeDisplay {
    pub file_name: String,
    pub file_size: u64,
    pub file_count: usize,
    pub is_directory: bool,
    pub depth: u32,
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
}

impl dir_model::DirectoryModel {
    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        roles.insert(Roles::FileName as i32, QByteArray::from("fileName"));
        roles.insert(Roles::FileSize as i32, QByteArray::from("fileSize"));
        roles.insert(Roles::FileCount as i32, QByteArray::from("fileCount"));
        roles.insert(Roles::IsDirectory as i32, QByteArray::from("isDirectory"));
        roles.insert(Roles::Depth as i32, QByteArray::from("depth"));
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
            }
        }
        QVariant::default()
    }

    pub fn load_tree(mut self: Pin<&mut Self>) {
        if let Ok(lock) = crate::bridge::LATEST_TREE.read() {
            if let Some(tree) = lock.as_ref() {
                let mut new_items = Vec::new();
                
                if let Some(root_id) = tree.get_root() {
                    let root_data = tree.get_data(root_id).unwrap();
                    new_items.push(NodeDisplay {
                        file_name: root_data.name.to_string(),
                        file_size: root_data.size,
                        file_count: 0,
                        is_directory: true,
                        depth: 0,
                    });
                    
                    for child_id in tree.get_children(root_id) {
                        let child_data = tree.get_data(child_id).unwrap();
                        new_items.push(NodeDisplay {
                            file_name: child_data.name.to_string(),
                            file_size: child_data.size,
                            file_count: 0,
                            is_directory: child_data.flags.contains(topograph_core::NodeFlags::IS_DIRECTORY),
                            depth: 1,
                        });
                    }
                }

                unsafe {
                    self.as_mut().begin_reset_model();
                    self.as_mut().rust_mut().items = new_items;
                    self.as_mut().end_reset_model();
                }
            }
        }
    }
}
