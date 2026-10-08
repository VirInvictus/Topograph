#include "treemap_view.h"

#include <QtQuick/QSGGeometry>
#include <QtQuick/QSGGeometryNode>
#include <QtQuick/QSGVertexColorMaterial>
#include <QtQml/qqml.h>
#include <cstring>
#include <vector>

// Vertex plumbing declared by the cxx-qt bridge's extern "Rust" block
// (generated scan_bridge.cxx.h) and defined in the generated bridge
// sources. Declared directly here because including the full generated
// header would drag the QObject declarations into this translation unit;
// the signatures below match the generated ones verbatim.
std::size_t treemap_rebuild(float width, float height, float cushion_height,
                            float ambient, float light_angle, float padding) noexcept;
void treemap_copy_vertices(std::uint8_t *out, std::size_t len) noexcept;
std::size_t treemap_tile_count() noexcept;

namespace {

// Sanity: the byte buffer from Rust is consumed as QSGGeometry's
// ColoredPoint2D array (two floats plus four color bytes).
static_assert(sizeof(QSGGeometry::ColoredPoint2D) == 12, "vertex layout drift");

} // namespace

TreemapView::TreemapView(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
}

void TreemapView::setCushionHeight(float value)
{
    if (m_cushionHeight == value)
        return;
    m_cushionHeight = value;
    emit optionsChanged();
    reload();
}

void TreemapView::setAmbient(float value)
{
    if (m_ambient == value)
        return;
    m_ambient = value;
    emit optionsChanged();
    reload();
}

void TreemapView::setLightAngle(float value)
{
    if (m_lightAngle == value)
        return;
    m_lightAngle = value;
    emit optionsChanged();
    reload();
}

void TreemapView::setPadding(float value)
{
    if (m_padding == value)
        return;
    m_padding = value;
    emit optionsChanged();
    reload();
}

void TreemapView::reload()
{
    refreshBuffer();
    update();
}

void TreemapView::refreshBuffer()
{
    const std::size_t count = treemap_rebuild(
        float(width()), float(height()), m_cushionHeight, m_ambient, m_lightAngle, m_padding);
    m_bytes.resize(count);
    if (count > 0)
        treemap_copy_vertices(m_bytes.data(), count);
    // Laid tiles from the bridge (each tile emits a grid of triangles), not
    // the vertex count.
    const int tiles = int(treemap_tile_count());
    if (tiles != m_tiles) {
        m_tiles = tiles;
        emit verticesChanged();
    }
}

void TreemapView::geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry)
{
    QQuickItem::geometryChange(newGeometry, oldGeometry);
    if (newGeometry.size() != oldGeometry.size())
        reload();
}

QSGNode *TreemapView::updatePaintNode(QSGNode *old, UpdatePaintNodeData *data)
{
    Q_UNUSED(data);

    if (m_bytes.empty()) {
        delete old;
        return nullptr;
    }

    const int vertexCount = int(m_bytes.size() / sizeof(QSGGeometry::ColoredPoint2D));

    QSGGeometryNode *node;
    if (!old) {
        node = new QSGGeometryNode;
        node->setMaterial(new QSGVertexColorMaterial);
        node->setFlag(QSGNode::OwnsMaterial);
        node->setGeometry(
            new QSGGeometry(QSGGeometry::defaultAttributes_ColoredPoint2D(), vertexCount));
        node->setFlag(QSGNode::OwnsGeometry);
    } else {
        node = static_cast<QSGGeometryNode *>(old);
        if (node->geometry()->vertexCount() != vertexCount)
            node->geometry()->allocate(vertexCount);
    }

    std::memcpy(node->geometry()->vertexData(), m_bytes.data(), m_bytes.size());
    node->markDirty(QSGNode::DirtyGeometry);
    return node;
}

namespace {
const bool treemapViewRegistered = [] {
    // Separate URI from the generated com.topograph module: the generated
    // module's qmldir wins type resolution for its own URI, so hand-written
    // C++ types register cleanly under their own.
    qmlRegisterType<TreemapView>("com.topograph.treemap", 1, 0, "TreemapView");
    return true;
}();
} // namespace

void topograph_treemap_force_link()
{
    // The registration above lives in this translation unit's static
    // initializer; referencing it from the Rust side keeps the linker from
    // discarding the object file (see bridge.rs force_link).
    Q_UNUSED(treemapViewRegistered);
}
