use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

mod bridge;
mod directory_model;

fn main() {
    // Prevent linker from stripping modules by referencing them
    bridge::force_link();
    directory_model::force_link();
    // Create the QGuiApplication. Qt sees no argv here; the optional
    // command-line path reaches QML through the bridge's initial_path
    // property, which reads std::env::args when the bridge constructs.
    let mut app = QGuiApplication::new();

    // Initialize the QML engine
    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/com/topograph/qml/main.qml"));
    }

    // Start the Qt event loop
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
