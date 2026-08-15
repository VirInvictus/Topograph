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
                
                delegate: Item {
                    width: treeView.width
                    height: 24
                    
                    RowLayout {
                        anchors.fill: parent
                        
                        // Indentation based on depth
                        Item {
                            Layout.preferredWidth: model.depth * 20
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
        function onIsScanningChanged() {
            if (!bridge.isScanning && bridge.progressText === "Scan complete.") {
                dirModel.loadTree()
            }
        }
    }
}
