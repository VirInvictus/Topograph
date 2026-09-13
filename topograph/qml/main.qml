import QtQuick 2.15
import QtQuick.Controls 2.15
import QtQuick.Layouts 1.15
import com.topograph 1.0

ApplicationWindow {
    visible: true
    width: 1024
    height: 768
    title: "Topograph"
    color: "#181616" // Kanagawa Dragon Background

    ScanBridge {
        id: bridge
    }

    Timer {
        interval: 16 // ~60fps
        running: bridge.isScanning
        repeat: true
        onTriggered: bridge.updateMetrics()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20

        RowLayout {
            Layout.fillWidth: true
            
            TextField {
                id: pathInput
                Layout.fillWidth: true
                text: "/"
                color: "#c5c9c5" // Dragon Foreground
                background: Rectangle {
                    color: "#282727" // Dragon Surface
                    radius: 4
                }
            }

            Button {
                text: bridge.isScanning ? "Cancel" : "Scan"
                onClicked: {
                    if (bridge.isScanning) {
                        bridge.cancelScan()
                    } else {
                        bridge.startScan(pathInput.text)
                    }
                }
                background: Rectangle {
                    color: bridge.isScanning ? "#c4746e" : "#87a987" // Dragon Red vs Green
                    radius: 4
                }
                contentItem: Text {
                    text: parent.text
                    color: "#c5c9c5" // Dragon Foreground
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: bridge.isScanning || bridge.progressText !== ""
            
            Text {
                text: bridge.progressText
                color: "#8ba4b0" // Dragon Blue
                font.pixelSize: 14
            }

            ProgressBar {
                Layout.fillWidth: true
                Layout.leftMargin: 10
                Layout.rightMargin: 10
                indeterminate: bridge.isScanning
                visible: bridge.isScanning
                background: Rectangle {
                    color: "#282727" // Dragon Surface
                    radius: 2
                }
                contentItem: Item {
                    Rectangle {
                        id: progressRect
                        width: parent.width * 0.3
                        height: parent.height
                        color: "#c4746e" // Dragon Red
                        radius: 2
                        NumberAnimation on x {
                            from: 0
                            to: progressRect.parent.width * 0.7
                            duration: 1000
                            loops: Animation.Infinite
                            running: bridge.isScanning
                        }
                    }
                }
            }

            Text {
                text: bridge.speedText
                color: "#8ea4a2" // Dragon Aqua
                font.pixelSize: 14
            }
        }

        // Sort header
        RowLayout {
            Layout.fillWidth: true
            visible: treeView.count > 0

            Text {
                text: "Sort"
                color: "#625e5a" // Dragon Muted
                font.pixelSize: 12
            }

            Button {
                text: "Size"
                onClicked: treeView.sortPreserving("size", dirModel.sortKey === "size" ? !dirModel.sortDescending : true)
                background: Rectangle {
                    color: dirModel.sortKey === "size" ? "#282727" : "transparent" // Dragon Surface
                    radius: 4
                }
                contentItem: Text {
                    text: parent.text
                    color: dirModel.sortKey === "size" ? "#8ea4a2" : "#625e5a" // Dragon Aqua vs Muted
                    font.pixelSize: 12
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Button {
                text: "Name"
                onClicked: treeView.sortPreserving("name", dirModel.sortKey === "name" ? !dirModel.sortDescending : false)
                background: Rectangle {
                    color: dirModel.sortKey === "name" ? "#282727" : "transparent" // Dragon Surface
                    radius: 4
                }
                contentItem: Text {
                    text: parent.text
                    color: dirModel.sortKey === "name" ? "#8ea4a2" : "#625e5a" // Dragon Aqua vs Muted
                    font.pixelSize: 12
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }

            Button {
                text: "Count"
                onClicked: treeView.sortPreserving("count", dirModel.sortKey === "count" ? !dirModel.sortDescending : false)
                background: Rectangle {
                    color: dirModel.sortKey === "count" ? "#282727" : "transparent" // Dragon Surface
                    radius: 4
                }
                contentItem: Text {
                    text: parent.text
                    color: dirModel.sortKey === "count" ? "#8ea4a2" : "#625e5a" // Dragon Aqua vs Muted
                    font.pixelSize: 12
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }

        // Content Area
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            color: "#181616" // Dragon Background
            radius: 8
            clip: true
            
            ListView {
                id: treeView
                anchors.fill: parent
                anchors.margins: 8
                model: dirModel
                focus: true
                highlightMoveDuration: 0
                highlight: Rectangle {
                    color: "#282727" // Dragon Surface
                    radius: 4
                }

                // Identity of the current row, carried across the full model
                // resets that expandRow/collapseRow/sortBy publish (every one
                // of them clears currentIndex).
                property int restoreIndex: -1
                property string restoreName: ""
                property int restoreDepth: -1

                function preserveCurrent() {
                    restoreIndex = currentIndex
                    restoreName = currentItem ? currentItem.rowName : ""
                    restoreDepth = currentItem ? currentItem.rowDepth : -1
                }

                // Re-finds the preserved row by name+depth, scanning outward
                // from its old index. Delegates only exist for the viewport
                // neighbourhood, so a row the operation moved out of that
                // window falls back to sitting at the old index position.
                function findRow(name, depth, hint) {
                    const reach = 120
                    for (let offset = 0; offset <= reach; offset++) {
                        let candidate = hint - offset
                        if (candidate >= 0) {
                            let item = itemAtIndex(candidate)
                            if (item && item.rowName === name && item.rowDepth === depth)
                                return candidate
                        }
                        candidate = hint + offset
                        if (offset > 0 && candidate < count) {
                            let item = itemAtIndex(candidate)
                            if (item && item.rowName === name && item.rowDepth === depth)
                                return candidate
                        }
                    }
                    return -1
                }

                function restoreCurrent() {
                    if (restoreIndex < 0 || count === 0)
                        return
                    forceLayout()
                    let row = findRow(restoreName, restoreDepth, restoreIndex)
                    if (row < 0)
                        row = Math.min(restoreIndex, count - 1)
                    currentIndex = row
                    positionViewAtIndex(row, ListView.Contain)
                    forceActiveFocus()
                }

                function expandPreserving(row) {
                    preserveCurrent()
                    dirModel.expandRow(row)
                    restoreCurrent()
                }

                function collapsePreserving(row) {
                    preserveCurrent()
                    dirModel.collapseRow(row)
                    restoreCurrent()
                }

                function sortPreserving(key, descending) {
                    preserveCurrent()
                    dirModel.sortBy(key, descending)
                    restoreCurrent()
                }

                Keys.onLeftPressed: if (currentIndex >= 0) collapsePreserving(currentIndex)
                Keys.onRightPressed: if (currentIndex >= 0) expandPreserving(currentIndex)

                delegate: Item {
                    property string rowName: model.fileName
                    property int rowDepth: model.depth
                    width: treeView.width
                    height: 24

                    RowLayout {
                        anchors.fill: parent

                        // Indentation based on depth
                        Item {
                            Layout.preferredWidth: model.depth * 20
                        }

                        Text {
                            text: model.isDirectory ? (model.expanded ? "▾" : "▸") : ""
                            color: "#8ea4a2" // Dragon Aqua
                            font.pixelSize: 12
                            Layout.preferredWidth: 14
                        }

                        Text {
                            text: model.isDirectory ? "📁" : "📄"
                            font.pixelSize: 14
                        }

                        Text {
                            text: model.fileName
                            color: "#c5c9c5" // Dragon Foreground
                            font.pixelSize: 14
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }

                        Text {
                            text: {
                                let mb = model.fileSize / (1024 * 1024);
                                return mb.toFixed(2) + " MB";
                            }
                            color: "#625e5a" // Dragon Muted
                            font.pixelSize: 14
                            horizontalAlignment: Text.AlignRight
                        }

                        Rectangle {
                            Layout.preferredWidth: 120
                            Layout.preferredHeight: 6
                            Layout.alignment: Qt.AlignVCenter
                            color: "#282727" // Dragon Surface
                            radius: 3
                            clip: true

                            Rectangle {
                                width: parent.width * Math.min(model.percent, 100) / 100
                                height: parent.height
                                color: "#8ea4a2" // Dragon Aqua
                                radius: 3
                            }
                        }

                        Text {
                            text: model.percent.toFixed(1) + "%"
                            color: "#625e5a" // Dragon Muted
                            font.pixelSize: 12
                            horizontalAlignment: Text.AlignRight
                            Layout.preferredWidth: 48
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            treeView.forceActiveFocus()
                            treeView.currentIndex = index
                            if (model.isDirectory) {
                                if (model.expanded)
                                    treeView.collapsePreserving(index)
                                else
                                    treeView.expandPreserving(index)
                            }
                        }
                    }
                }
            }

            Text {
                anchors.centerIn: parent
                text: bridge.isScanning ? "Scanning..." : "Ready."
                color: "#625e5a" // Dragon Muted
                font.pixelSize: 24
                visible: treeView.count === 0
            }
        }
    }

    DirectoryModel {
        id: dirModel
    }

    Connections {
        target: bridge
        function onScanFinished() {
            dirModel.loadTree()
            treeView.forceActiveFocus()
        }
    }
}
