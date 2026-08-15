fn main() {
    cxx_qt_build::CxxQtBuilder::new()
        .qml_module(cxx_qt_build::QmlModule {
            uri: "com.topograph",
            version_major: 1,
            version_minor: 0,
            rust_files: &["src/bridge.rs", "src/directory_model.rs"],
            qml_files: &["qml/main.qml"],
            ..Default::default()
        })
        .build();
}
