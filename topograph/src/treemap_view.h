#pragma once

#include <QtQuick/QQuickItem>
#include <vector>

// The treemap renderer (roadmap Phase 9): a QQuickItem whose scene-graph
// node is one QSGGeometryNode of triangles built from the vertex buffer the
// Rust core computes (topograph-core::treemap). All layout, cushion, and
// lighting math lives in Rust; this class only forwards the current item
// size plus the exposed lighting parameters, refills its byte buffer from
// the bridge, and hands the bytes to the GPU as colored-point geometry.
class TreemapView : public QQuickItem
{
    Q_OBJECT
    Q_PROPERTY(float cushionHeight READ cushionHeight WRITE setCushionHeight NOTIFY optionsChanged)
    Q_PROPERTY(float ambient READ ambient WRITE setAmbient NOTIFY optionsChanged)
    Q_PROPERTY(float lightAngle READ lightAngle WRITE setLightAngle NOTIFY optionsChanged)
    Q_PROPERTY(float padding READ padding WRITE setPadding NOTIFY optionsChanged)
    Q_PROPERTY(int tileCount READ tileCount NOTIFY verticesChanged)

public:
    explicit TreemapView(QQuickItem *parent = nullptr);

    // Defaults mirror topograph_core::treemap::TreemapOptions::default().
    float cushionHeight() const { return m_cushionHeight; }
    float ambient() const { return m_ambient; }
    float lightAngle() const { return m_lightAngle; }
    float padding() const { return m_padding; }
    int tileCount() const { return m_tiles; }

    void setCushionHeight(float value);
    void setAmbient(float value);
    void setLightAngle(float value);
    void setPadding(float value);

public slots:
    // Rebuilds the buffer from the currently displayed tree at the current
    // item size. QML calls this on scan completion; size and option changes
    // rebuild automatically.
    void reload();

signals:
    void optionsChanged();
    void verticesChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *old, UpdatePaintNodeData *data) override;
    void geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry) override;

private:
    // Rebuilds m_bytes via the Rust bridge (GUI thread only; the render
    // thread only reads m_bytes inside updatePaintNode).
    void refreshBuffer();

    float m_cushionHeight = 0.7f;
    float m_ambient = 0.55f;
    float m_lightAngle = 3.92699f; // 5*pi/4, toward the top-left
    float m_padding = 3.0f;
    int m_tiles = 0;
    std::vector<unsigned char> m_bytes;
};

// Defined in treemap_view.cpp; referenced from the Rust bridge's force_link
// so the linker keeps this object file (and its QML registration) alive.
extern "C++" void topograph_treemap_force_link();
