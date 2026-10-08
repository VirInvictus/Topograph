#pragma once

#include <QtCore/QString>

// Headless GUI selftest driver: walks the loaded QML tree by class name,
// drives the app through property writes and method invokes (no synthetic
// input), pumps the event loop, and captures frames via
// QQuickWindow::grabWindow. Called from the topograph-selftest bin; the
// return value is the process exit code (0 = every step passed).
//
// `pixelChecks` gates the treemap's pixel-level assertions: the software
// scene graph does not draw raw QSGGeometryNode items, so a software
// fallback run skips the map's image comparisons (printing SKIP lines)
// while still verifying the data pipeline, tile count, and resize size.
// Codes 1/2 additionally mean "no window / nothing rendered at all", the
// caller's cue to retry on another graphics path.
int topograph_selftest_capture(const QString &outDir, const QString &scanPath, bool pixelChecks);
