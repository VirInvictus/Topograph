use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

mod bridge;

fn main() {
    // Create the QGuiApplication, passing in arguments from the environment
    let mut app = QGuiApplication::new();

    // Initialize the QML engine
    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/main.qml"));
    }

    // Start the Qt event loop
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
