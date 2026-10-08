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
        // QQuickItem / QSGGeometryNode for the treemap renderer.
        .qt_module("Quick")
        // moc for the hand-written TreemapView QObject, plus its
        // implementation file compiled by the same cc build that compiles
        // the generated CXX-Qt sources (so Qt include paths are shared).
        .qobject_header("src/treemap_view.h")
        .cc_builder(|cc| {
            cc.include("src");
            cc.file("src/treemap_view.cpp");
            cc.file("src/selftest_support.cpp");
        })
        .build();
}
