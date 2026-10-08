#include "selftest_support.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDeadlineTimer>
#include <QtCore/QDir>
#include <QtCore/QEventLoop>
#include <QtCore/QMetaObject>
#include <QtCore/QVariant>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>
#include <QtGui/QWindow>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>

#include <cstdio>
#include <cstring>

namespace {

// The QML object graph is walked by generated class name: the cxx-qt bridge
// types (ScanBridge) and the hand-written TreemapView are findable from the
// window without QML ids or objectNames.
QObject *findByClassName(QObject *root, const char *needle)
{
    if (std::strstr(root->metaObject()->className(), needle))
        return root;
    const auto children = root->findChildren<QObject *>();
    for (QObject *child : children) {
        if (std::strstr(child->metaObject()->className(), needle))
            return child;
    }
    return nullptr;
}

// Process events for roughly `ms` so bindings, the 60fps metrics Timer, and
// scene-graph updates run without an exec() loop.
void pump(int ms)
{
    QDeadlineTimer deadline(ms);
    while (!deadline.hasExpired())
        QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
}

bool isVaried(const QImage &img)
{
    if (img.isNull() || img.width() < 2 || img.height() < 2)
        return false;
    const QRgb first = img.pixel(0, 0);
    for (int y = 0; y < img.height(); y += 8) {
        for (int x = 0; x < img.width(); x += 8) {
            if (img.pixel(x, y) != first)
                return true;
        }
    }
    return false;
}

struct Grab {
    bool ok;
    QImage image;
};

Grab grab(QQuickWindow *window, const QString &path)
{
    const QImage img = window->grabWindow();
    if (img.isNull()) {
        std::fprintf(stderr, "FAIL grab %s: null image\n", qPrintable(path));
        return {false, QImage()};
    }
    if (!img.save(path)) {
        std::fprintf(stderr, "FAIL grab %s: save failed\n", qPrintable(path));
        return {false, QImage()};
    }
    if (!isVaried(img)) {
        std::fprintf(stderr, "FAIL grab %s: uniform image (%dx%d)\n", qPrintable(path),
                     img.width(), img.height());
        return {false, img};
    }
    std::fprintf(stdout, "grab %s (%dx%d)\n", qPrintable(path), img.width(), img.height());
    return {true, img};
}

// Sampled pixel-difference count between two captures of the same size.
int sampledDiff(const QImage &a, const QImage &b)
{
    if (a.size() != b.size())
        return INT_MAX;
    int diff = 0;
    for (int y = 0; y < a.height(); y += 4) {
        for (int x = 0; x < a.width(); x += 4) {
            if (a.pixel(x, y) != b.pixel(x, y))
                ++diff;
        }
    }
    return diff;
}

} // namespace

int topograph_selftest_capture(const QString &outDir, const QString &scanPath, bool pixelChecks)
{
    QDir().mkpath(outDir);
    pump(300); // let the engine finish loading the QML

    QQuickWindow *window = nullptr;
    const auto windows = QGuiApplication::topLevelWindows();
    for (QWindow *win : windows) {
        if ((window = qobject_cast<QQuickWindow *>(win)))
            break;
    }
    if (!window) {
        std::fprintf(stderr, "FAIL: no QQuickWindow among %lld top-level windows\n",
                     static_cast<long long>(windows.size()));
        return 1;
    }

    // The initial shell, before any scan. A blank/null grab here means the
    // graphics path produced nothing at all (the caller's retry cue).
    const Grab initial = grab(window, outDir + "/00-initial.png");
    if (!initial.ok)
        return 2;

    // Start the scan directly on the bridge qobject: Q_INVOKABLE startScan,
    // no synthetic input.
    QObject *bridge = findByClassName(window, "ScanBridge");
    if (!bridge) {
        std::fprintf(stderr, "FAIL: ScanBridge not found in the object tree\n");
        return 3;
    }
    if (!QMetaObject::invokeMethod(bridge, "startScan", Q_ARG(QString, scanPath))) {
        std::fprintf(stderr, "FAIL: startScan invoke failed\n");
        return 4;
    }

    // Wait for the completion summary (both totals, "apparent /") on the
    // progress line; the 60fps QML Timer advances it under processEvents.
    bool finished = false;
    QDeadlineTimer scanDeadline(60000);
    while (!scanDeadline.hasExpired()) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
        if (bridge->property("progressText").toString().contains(QLatin1String("apparent"))) {
            finished = true;
            break;
        }
    }
    if (!finished) {
        std::fprintf(stderr, "FAIL: scan did not complete, progress: %s\n",
                     qPrintable(bridge->property("progressText").toString()));
        return 5;
    }
    std::fprintf(stdout, "summary: %s\n", qPrintable(bridge->property("progressText").toString()));
    pump(400);

    // The scanned tree.
    const Grab tree = grab(window, outDir + "/01-tree.png");
    if (!tree.ok)
        return 6;

    // Flip the view; the Loader instantiates TreemapView, whose
    // Component.onCompleted reload must have laid tiles by now.
    window->setProperty("viewMode", QLatin1String("treemap"));
    pump(600);
    auto *map = qobject_cast<QQuickItem *>(findByClassName(window, "TreemapView"));
    if (!map) {
        std::fprintf(stderr, "FAIL: TreemapView not found after the mode switch\n");
        return 7;
    }
    const int tiles = map->property("tileCount").toInt();
    if (tiles < 4) {
        std::fprintf(stderr, "FAIL: expected several tiles from the fixture, got %d\n", tiles);
        return 8;
    }
    std::fprintf(stdout, "tiles: %d\n", tiles);

    Grab treemap{true, QImage()};
    if (pixelChecks) {
        treemap = grab(window, outDir + "/02-treemap.png");
        if (!treemap.ok)
            return 9;
    } else {
        std::fprintf(stdout,
                     "SKIP 02-treemap.png: the software scene graph does not draw "
                     "QSGGeometryNode items\n");
    }

    // Cushion off through the same property path the slider drives; the
    // shading must visibly change (pixel-verified on GL paths only).
    map->setProperty("cushionHeight", 0.0);
    pump(400);
    Grab flat{true, QImage()};
    if (pixelChecks) {
        flat = grab(window, outDir + "/03-flat.png");
        if (!flat.ok)
            return 10;
        if (sampledDiff(treemap.image, flat.image) == 0) {
            std::fprintf(stderr, "FAIL: cushionHeight 0 did not change the render\n");
            return 11;
        }
    } else {
        std::fprintf(stdout, "SKIP 03-flat.png and the cushion diff (software renderer)\n");
    }

    // Dynamic resize: geometryChange triggers the Rust relayout and the
    // repaint at the new size; the capture itself must come back 1400x900.
    window->resize(QSize(1400, 900));
    pump(700);
    const Grab resized = grab(window, outDir + "/04-resized.png");
    if (!resized.ok)
        return 12;
    if (resized.image.width() != 1400 || resized.image.height() != 900) {
        std::fprintf(stderr, "FAIL: resize did not take: %dx%d\n", resized.image.width(),
                     resized.image.height());
        return 13;
    }

    std::fprintf(stdout, "SELFTEST OK%s (%s)\n", pixelChecks ? "" : " (degraded: software)",
                 qPrintable(QDir(outDir).absolutePath()));
    return 0;
}
